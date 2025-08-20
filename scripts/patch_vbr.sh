#!/bin/bash
# -*- coding: utf8 -*-

# Helper script to patch the built VBR into
# a target partition or disk image
#
# The goal is to skip the first 62 bytes of
# the source and target VBRs - which might
# contain the extened BIOS parameter block.
# - the key assumption is that modern
# partition managers populate the first
# 62 bytes with `eb 3c 90 <extended parameters>`
# - while our VBR shouldn't have to depend
# on the EBPB (see boot/src/vbr.asm), it would
# *suck* if we lazily copied the entire VBR
# onto the target volume and overwrite the 
# EBPB with placeholder values

# Capture current working directory
# in case the script gets invoked
# by the Makefile
CWD="$(pwd)"
DEF_VOLUME="${CWD}/build/boot.img"
SRC_VBR="${CWD}/build/vbr.bin"
LOGFILE="${CWD}/.patch_vbr-$$.log"

# Allow write to real devices
IS_DEVICE=0

# Bypass safety checks
FORCE=0

# Do not make backup of target VBR
NO_BACKUP=0

# Helper routines

# - show help screen
show_help_screen() {
    printf "patch_vbr.sh - Patch target volume with VBR at 'build/vbr.bin'\n"
    printf "Usage: ./patch_vbr.sh [arguments] <target-volume>\n"
    printf "Switches:\n"
    printf "    -h / --help\t\t Show this screen and exit\n"
    printf "    -d / --is-device\t Allow writes to real devices\n"
    printf "    -f / --force\t Bypass all safety checks\n"
    printf "                \t (e.g. enforcing equal jump vectors)\n"
    printf "    --no-backup \t Do not make backup of target VBR\n"
    printf "\n"
    printf "This script should NOT be executed on its own, and\n"
    printf "should instead be executed during the build process.\n"
    printf "(see 'Makefile' and 'README.md')\n"
    printf "\n"
}

# Lazily parse arguments
for ARG in "$@"; do
    # - let's assume that dashed wildcards
    # get parsed before the general wildcards
    case "$ARG" in
        -h | --help)
            show_help_screen
            exit 0
            ;;
        -d | --is-device)
            IS_DEVICE=1
            ;;
        -f | --force)
            FORCE=1
            ;;
        --no-backup)
            NO_BACKUP=1
            ;;
        -*)
            echo " E: Unrecognized argument '$ARG'" >&2
            echo "Use '-h' or '--help' for more information." >&2
            exit 1
            ;;
        *)
            VOLUME="$ARG"
            break 
    esac
done    

# Check whether the source VBR exists
if [ ! -e "$SRC_VBR" ]; then
    echo " E: Cannot find source VBR at '${SRC_VBR}' - exiting..." >&2
    echo "Run 'make' inside the project directory to resolve this issue."
    exit 1
fi

# Initialize log file
HAS_LOGFILE="$(test -e "$LOGFILE" && echo "$LOGFILE")"

{
    [ -n "$HAS_LOGFILE" ] && echo;
    echo " I: patch_vbr.sh - invocation at $(date +'%F %T%z')";
} >>"$LOGFILE"

if [ "$FORCE" -eq 1 ]; then
    echo " W: Safety checks will be bypassed" | tee -a "$LOGFILE" >&2
fi

# - obtain magic bytes from source VBR and calculate jump vector
echo " I: Reading from source VBR..." >> "$LOGFILE"

IN_MAGIC=( $(dd if="$SRC_VBR" bs=1 count=3 2>>"$LOGFILE" | \
    hexdump -C | awk '{print $2" "$3" "$4}') )
IN_VECTOR="$(("0x${IN_MAGIC[1]}" + 2))"

echo " I: Magic sequence in source is '${IN_MAGIC[0]} ${IN_MAGIC[1]} ${IN_MAGIC[2]}'" >>"$LOGFILE"

# - check whether a valid jump instruction is present
if [ "${IN_MAGIC[0]}" != "eb" ] || [ "${IN_MAGIC[2]}" != "90" ]; then
    if [ "$FORCE" -eq 1 ]; then
        echo " W: Jump instruction not found in source VBR" | tee -a "$LOGFILE" >&2
    else
        echo " E: Jump instruction not found in source VBR - exiting..." | \
            tee -a "$LOGFILE" >&2

        exit 1
    fi
fi

# Check whether the target volume exists
VOLUME="${VOLUME:-${DEF_VOLUME}}"

if [ ! -e "$VOLUME" ]; then
    echo " E: Could not find volume '${VOLUME}' - exiting..." | tee -a "$LOGFILE" >&2
    exit 1
fi

# Check whether the target volume is a
# block device (possibly real hardware)
# - for our own sanity, accept character devices as well
if { [ -b "$VOLUME" ] || [ -c "$VOLUME" ]; } && [ "$IS_DEVICE" -eq 0 ]; then
    if [ "$FORCE" -eq 1 ]; then
        {
            echo " W: Volume '${VOLUME}' is a real device";
            echo " W: ('-d'/'--is-device' not provided)";
        } | \
            tee -a "$LOGFILE" >&2
    else
        echo " E: Volume '$VOLUME' is a real device - exiting..." | tee -a "$LOGFILE" >&2

        echo "Use '-d' or '--is-device' to override this." >&2
        exit 1
    fi
fi

# - fall-through if -b / --is-block is provided
# - for our own sanity, accept character devices as well
if { [ -b "$VOLUME" ] || [ -c "$VOLUME" ]; } && [ "x$UID" != "x0" ]; then
    # - cannot do jack without privileges
    echo " E: Cannot write to block device without privileges - exiting..." | \
        tee -a "$LOGFILE" >&2

    echo "Try again as superuser to resolve this issue." >&2
    exit 1
fi

# - fall-through on success
{
    echo " I: Will write to volume '${VOLUME}'...";
    [ "$IS_DEVICE" -eq 1 ] && echo " I: (target volume is a real device)";
} | tee -a "$LOGFILE"

# Obtain magic bytes from target VBR and calculate jump vector
OUT_MAGIC=( $(dd if="$VOLUME" bs=1 count=3 2>>"$LOGFILE" | \
    hexdump -C | awk '{print $2" "$3" "$4}') )
OUT_VECTOR="$(("0x${OUT_MAGIC[1]}" + 2))"

printf " I: Magic sequence in target is '%s %s %s'\n" \
    "${OUT_MAGIC[0]}" \
    "${OUT_MAGIC[1]}" \
    "${OUT_MAGIC[2]}" \
    >>"$LOGFILE"

# - check whether a valid jump instruction is present
if [ "${OUT_MAGIC[0]}" != "eb" ] || [ "${OUT_MAGIC[2]}" != "90" ]; then
    if [ "$FORCE" -eq 1 ]; then
        echo " W: Jump instruction not found in target VBR" | tee -a "$LOGFILE" >&2
    else
        echo " E: Jump instruction not found in target VBR - exiting..." | \
            tee -a "$LOGFILE" >&2

        exit 1
    fi
fi

# Check whether the input and output vectors match
# - if they don't, we can't safely proceed, as label
# offsets in the source VBR may be hard-coded
# (IP-relative addressing consumes precious space)
if [ "$IN_VECTOR" -ne "$OUT_VECTOR" ] && [ "$FORCE" -eq 0 ]; then
    {
        echo " E: Jump vectors in source and target do not match - exiting...";
        echo " E: (source: 0x${IN_MAGIC[1]}+2 != target: 0x${OUT_MAGIC[1]}+2)";
    } | tee -a "$LOGFILE" >&2

    exit 1
elif [ "$IN_VECTOR" -ne "$OUT_VECTOR" ] && [ "$FORCE" -eq 1 ]; then
    {
        echo " W: Jump vectors in source and target do not match";
        echo " W: (source: 0x${IN_MAGIC[1]}+2 != target: 0x${OUT_MAGIC[1]}+2)";
    } | tee -a "$LOGFILE" >&2
fi


# ---- Main routine ---- #

# - make backup of target VBR
OUT_BAK="${CWD}/target-$(date +'%Y%m%d-%H%M%S%z').bak"

if [ "$NO_BACKUP" -eq 1 ]; then
    echo " W: Will not make backup of target VBR" | tee -a "$LOGFILE" >&2
else
    echo " I: Making backup of target VBR..." | tee -a "$LOGFILE"
    dd  if="$VOLUME" of="$OUT_BAK" \
        bs=1 count=512 2>>"$LOGFILE"

    if [ "$?" -ne 0 ]; then
        echo " W: Failed to make backup of target VBR" | tee -a "$LOGFILE" >&2
    fi
fi

# - copy from source, skipping the EBPB
IN_COUNT="$((512 - "$IN_VECTOR"))"

{
    echo " I: Copying source VBR to target volume...";
    echo " I: (copying $IN_COUNT bytes)";
} | tee -a "$LOGFILE"

dd  if="$SRC_VBR" of="$VOLUME" \
    bs=1 count="$IN_COUNT" \
    conv=notrunc skip="$IN_VECTOR" \
    seek="$OUT_VECTOR" 2>>"$LOGFILE"

if [ "$?" -ne 0 ]; then
    echo " E: Failed to copy source VBR to target volume"
    exit 1
fi
