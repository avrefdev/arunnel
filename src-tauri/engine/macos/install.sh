#!/bin/sh
set -e
PATH=/usr/bin:/bin:/usr/sbin:/sbin
S="$1"
U="$2"
R="/Library/Application Support/Arunnel"
P=/Library/LaunchDaemons/app.arunnel.apply.plist
launchctl bootout system/app.arunnel.apply 2>/dev/null || true
mkdir -p "$R/engine" "$R/ctl"
for f in tpws apply.sh discord.txt roblox.txt names.txt version; do
  cp "$S/$f" "$R/engine/$f"
done
chown -R root:wheel "$R"
chmod 755 "$R" "$R/engine" "$R/engine/tpws" "$R/engine/apply.sh"
chmod 644 "$R/engine/discord.txt" "$R/engine/roblox.txt" "$R/engine/names.txt" "$R/engine/version"
xattr -cr "$R/engine" 2>/dev/null || true
chown "$U" "$R/ctl"
chmod 755 "$R/ctl"
cp "$S/app.arunnel.apply.plist" "$P"
chown root:wheel "$P"
chmod 644 "$P"
touch /var/run/arunnel.boot
launchctl bootstrap system "$P"
