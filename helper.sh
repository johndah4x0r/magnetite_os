#!/bin/sh
# -*- coding: utf-8 -*-

SRC="$(cat -)"
DEST="$1"

if [ "$DEST" -ot "$SRC" ] || [ ! -f "$DEST" ]; then
    cp -p "$SRC" "$DEST"
    exit "$?"
fi
