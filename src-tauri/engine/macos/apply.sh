#!/bin/sh

PATH=/usr/bin:/bin:/usr/sbin:/sbin
R="/Library/Application Support/Arunnel"
E="$R/engine"
REQ="$R/ctl/request"
ACK="$R/ack"
LOG="$R/engine.log"
PID=/var/run/arunnel-tpws.pid
SIG=/var/run/arunnel-tpws.sig
TOK=/var/run/arunnel-pf.token
BOOT=/var/run/arunnel.boot
ANCHOR=com.apple/arunnel
PORT=10443
BEGIN="# >>> Arunnel (otomatik eklendi, elle düzenlemeyin)"
END="# <<< Arunnel"
ZONES='(^|\.)(discord\.com|discord\.gg|discord\.media|discordapp\.com|discordapp\.net|discordcdn\.com|discordstatus\.com|dis\.gd)$'

log() {
  echo "$(date '+%H:%M:%S') $*" >> "$LOG"
}

profile_args() {
  case "$1" in
    0) echo "--split-pos=1,midsld --disorder" ;;
    1) echo "--tlsrec=sniext --split-pos=1,midsld" ;;
    2) echo "--split-pos=1,sniext+1,host+1,midsld-2,midsld,midsld+2,endhost-1 --disorder" ;;
    3) echo "--tlsrec=midsld --split-pos=1 --disorder" ;;
    4) echo "--split-pos=midsld --oob" ;;
    *) echo "--tlsrec=sniext+1 --split-pos=host+1 --oob" ;;
  esac
}

read_request() {
  seq=0
  on=0
  profile=0
  roblox=0
  autostart=0
  : > "$R/hosts.tmp"
  [ -f "$REQ" ] || return 0
  while IFS= read -r line; do
    key=${line%%=*}
    val=${line#*=}
    case "$key" in
      seq) case "$val" in *[!0-9]*|'') ;; *) seq=$val ;; esac ;;
      engine) [ "$val" = 1 ] && on=1 ;;
      roblox) [ "$val" = 1 ] && roblox=1 ;;
      autostart) [ "$val" = 1 ] && autostart=1 ;;
      profile) case "$val" in [0-5]) profile=$val ;; esac ;;
      host)
        ip=${val%% *}
        name=${val#* }
        echo "$ip" | grep -Eq '^[0-9]{1,3}(\.[0-9]{1,3}){3}$' || continue
        grep -qxF "$name" "$E/names.txt" || continue
        echo "$ip $name" >> "$R/hosts.tmp"
        ;;
    esac
  done < "$REQ"
}

write_hosts() {
  Z="$ZONES" awk -v B="$BEGIN" -v E="$END" '
    BEGIN { Z = ENVIRON["Z"] }
    $0 == B { inside = 1; next }
    $0 == E { inside = 0; next }
    inside { next }
    {
      line = $0
      sub(/#.*/, "", line)
      n = split(line, f, /[ \t]+/)
      k = 0
      bad = 0
      for (i = 1; i <= n; i++) {
        if (f[i] == "") continue
        k++
        if (k == 1) continue
        if (tolower(f[i]) !~ Z) bad = 1
      }
      if (k >= 2 && !bad) next
      out[++c] = $0
    }
    END {
      while (c > 0 && out[c] ~ /^[ \t]*$/) c--
      for (i = 1; i <= c; i++) print out[i]
    }
  ' /etc/hosts > "$R/hosts.new"
  if [ -s "$R/hosts.tmp" ]; then
    { echo; echo "$BEGIN"; cat "$R/hosts.tmp"; echo "$END"; } >> "$R/hosts.new"
  fi
  if ! cmp -s "$R/hosts.new" /etc/hosts; then
    cat "$R/hosts.new" > /etc/hosts
    dscacheutil -flushcache
    killall -HUP mDNSResponder 2>/dev/null
  fi
  rm -f "$R/hosts.new"
}

pf_on() {
  ips=$(cut -d' ' -f1 "$R/hosts.tmp" | sort -u | tr '\n' ' ')
  [ -n "$ips" ] || return 1
  {
    echo "table <arunnel> persist { $ips}"
    echo "rdr pass on lo0 inet proto tcp from !127.0.0.0/8 to <arunnel> port 443 -> 127.0.0.1 port $PORT"
    echo "pass out quick route-to (lo0 127.0.0.1) inet proto tcp from !127.0.0.0/8 to <arunnel> port 443 user { >root }"
    echo "block drop out quick inet proto udp from any to <arunnel> port 443"
  } > "$R/pf.rules"
  pfctl -a "$ANCHOR" -f "$R/pf.rules" >> "$LOG" 2>&1 || return 1
  if [ ! -s "$TOK" ]; then
    pfctl -E 2>&1 | sed -n 's/^Token : //p' > "$TOK"
  fi
  return 0
}

pf_off() {
  pfctl -a "$ANCHOR" -F all > /dev/null 2>&1
  if [ -s "$TOK" ]; then
    pfctl -X "$(cat "$TOK")" > /dev/null 2>&1
  fi
  rm -f "$TOK"
}

tpws_pid() {
  [ -s "$PID" ] || return 1
  p=$(cat "$PID")
  kill -0 "$p" 2>/dev/null || return 1
  echo "$p"
}

tpws_stop() {
  p=$(tpws_pid) && kill "$p" 2>/dev/null
  i=0
  while tpws_pid > /dev/null && [ $i -lt 20 ]; do
    sleep 0.1
    i=$((i + 1))
  done
  p=$(tpws_pid) && kill -9 "$p" 2>/dev/null
  rm -f "$PID" "$SIG"
}

tpws_start() {
  set -- --user=root --bind-addr=127.0.0.1 "--port=$PORT" --daemon "--pidfile=$PID" "--hostlist=$E/discord.txt"
  [ "$roblox" = 1 ] && set -- "$@" "--hostlist=$E/roblox.txt"
  set -- "$@" $(profile_args "$profile")
  "$E/tpws" "$@" >> "$LOG" 2>&1 || return 1
  sleep 0.4
  tpws_pid > /dev/null || return 1
  echo "$profile $roblox" > "$SIG"
}

apply_once() {
  read_request
  if [ ! -f "$BOOT" ]; then
    touch "$BOOT"
    if [ "$autostart" != 1 ]; then
      on=0
      : > "$R/hosts.tmp"
    fi
  fi
  write_hosts
  if [ "$on" != 1 ]; then
    tpws_stop
    pf_off
    echo "$seq ok" > "$ACK"
    return
  fi
  if ! pf_on; then
    log "pf kuralları yüklenemedi"
    tpws_stop
    pf_off
    echo "$seq err Güvenlik duvarı kuralları yüklenemedi" > "$ACK"
    return
  fi
  if ! tpws_pid > /dev/null || [ "$(cat "$SIG" 2>/dev/null)" != "$profile $roblox" ]; then
    tpws_stop
    if ! tpws_start; then
      log "tpws başlatılamadı"
      tpws_stop
      pf_off
      echo "$seq err Motor başlatılamadı" > "$ACK"
      return
    fi
  fi
  echo "$seq ok" > "$ACK"
}

uninstall() {
  : > "$R/hosts.tmp"
  write_hosts
  tpws_stop
  pf_off
  rm -f "$BOOT"
}

if [ "$1" = uninstall ]; then
  uninstall
  exit 0
fi

last=x
while :; do
  cur=$(sed -n 's/^seq=//p' "$REQ" 2>/dev/null)
  [ "$cur" = "$last" ] && break
  apply_once
  last=$cur
done
rm -f "$R/hosts.tmp"
exit 0
