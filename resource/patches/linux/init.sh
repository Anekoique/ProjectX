#!/bin/sh
mount -t devtmpfs devtmpfs /dev
exec /sbin/init
