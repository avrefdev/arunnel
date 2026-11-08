#!/bin/sh
PATH=/usr/bin:/bin:/usr/sbin:/sbin
R="/Library/Application Support/Arunnel"
P=/Library/LaunchDaemons/app.arunnel.apply.plist
launchctl bootout system/app.arunnel.apply 2>/dev/null
[ -f "$R/engine/apply.sh" ] && /bin/sh "$R/engine/apply.sh" uninstall
rm -f "$P" /var/run/arunnel-tpws.pid /var/run/arunnel-tpws.sig /var/run/arunnel-pf.token /var/run/arunnel.boot
rm -rf "$R"
exit 0
