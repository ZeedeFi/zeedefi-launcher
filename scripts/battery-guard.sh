#!/usr/bin/env bash
# Garde-fou batterie : sauvegarde le dépôt ZeedeFi sur GitHub puis éteint le PC
# lorsque la batterie atteint 8 %. Une sauvegarde silencieuse est aussi faite à
# 12 % pour limiter tout risque de perte.
#
# Modes :
#   --check        affiche l'état de la batterie et sort
#   --backup-only  commit + bundle + push, sans extinction (test manuel)
#   --once         un seul cycle de surveillance
#   --watch        boucle de surveillance (utilisé par le service systemd)
#
# Variables :
#   ZEEDEFI_BATTERY_THRESHOLD  seuil d'extinction (défaut 8)
#   ZEEDEFI_BATTERY_PRETHOLD   seuil de pré-sauvegarde (défaut 12)
#   ZEEDEFI_REPO              dépôt à sauvegarder

set -uo pipefail

REPO="${ZEEDEFI_REPO:-$HOME/Documents/Projet par défaut/zeedefi-launcher}"
CACHE_DIR="$HOME/.cache/zeedefi"
LOG="$CACHE_DIR/battery-guard.log"
BACKUP_DIR="$HOME/zeedefi-backups"
THRESHOLD="${ZEEDEFI_BATTERY_THRESHOLD:-8}"
PRETHOLD="${ZEEDEFI_BATTERY_PRETHOLD:-12}"
STATE_DONE="$CACHE_DIR/battery-guard.done"
STATE_PRE="$CACHE_DIR/battery-guard.pre"
INTERVAL="${ZEEDEFI_BATTERY_INTERVAL:-20}"

mkdir -p "$CACHE_DIR" "$BACKUP_DIR"

log() {
    printf '%s  %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*" | tee -a "$LOG" >&2
}

read_battery() {
    local dir capacity status
    for dir in /sys/class/power_supply/BAT*; do
        [ -r "$dir/capacity" ] || continue
        [ "$(cat "$dir/present" 2>/dev/null || echo 1)" = "1" ] || continue
        capacity="$(cat "$dir/capacity" 2>/dev/null)" || continue
        status="$(cat "$dir/status" 2>/dev/null || echo Unknown)"
        printf '%s %s\n' "$capacity" "$status"
        return 0
    done
    return 1
}

save_repository() {
    local reason="$1"
    local stamp bundle push_ok=0 attempt

    if [ ! -d "$REPO/.git" ]; then
        log "ERREUR : dépôt introuvable ($REPO)"
        return 1
    fi

    cd "$REPO" || return 1
    log "Sauvegarde du dépôt ($reason)"

    git add -A >>"$LOG" 2>&1
    if ! git diff --cached --quiet; then
        if git commit -m "Sauvegarde automatique — $reason ($(date '+%Y-%m-%d %H:%M'))" >>"$LOG" 2>&1; then
            log "Commit créé"
        else
            log "ERREUR : le commit a échoué"
        fi
    else
        log "Aucune modification à committer"
    fi

    stamp="$(date '+%Y%m%d-%H%M%S')"
    bundle="$BACKUP_DIR/zeedefi-launcher-$stamp.bundle"
    if git bundle create "$bundle" --all >>"$LOG" 2>&1; then
        log "Bundle local : $bundle"
    else
        log "ERREUR : création du bundle impossible"
    fi

    export GIT_TERMINAL_PROMPT=0
    for attempt in 1 2 3 4 5; do
        if timeout 90 git push origin main >>"$LOG" 2>&1; then
            push_ok=1
            log "Push GitHub réussi (tentative $attempt)"
            break
        fi
        log "Push échoué (tentative $attempt), nouvelle tentative dans 5 s"
        sleep 5
    done
    if [ "$push_ok" -ne 1 ]; then
        log "AVERTISSEMENT : push GitHub impossible — bundle local conservé"
    fi

    sync
    [ "$push_ok" -eq 1 ]
}

shutdown_now() {
    log "Extinction de la machine"
    sync
    if timeout 30 systemctl poweroff >>"$LOG" 2>&1; then
        return 0
    fi
    log "systemctl poweroff a échoué, tentative loginctl"
    if timeout 30 loginctl poweroff >>"$LOG" 2>&1; then
        return 0
    fi
    log "loginctl a échoué, tentative poweroff"
    timeout 30 poweroff >>"$LOG" 2>&1
}

check_once() {
    local battery capacity status
    battery="$(read_battery)" || {
        log "Aucune batterie détectée"
        return 0
    }
    capacity="${battery%% *}"
    status="${battery##* }"

    case "$capacity" in
        ''|*[!0-9]*) log "Capacité illisible : $battery"; return 0 ;;
    esac

    # Branché : on réarme les seuils pour une prochaine décharge.
    if [ "$status" != "Discharging" ]; then
        [ -e "$STATE_PRE" ] && rm -f "$STATE_PRE"
        [ -e "$STATE_DONE" ] && rm -f "$STATE_DONE"
        return 0
    fi

    log "Batterie ${capacity} % (${status})"

    if [ "$capacity" -le "$PRETHOLD" ] && [ ! -e "$STATE_PRE" ]; then
        log "Seuil de pré-sauvegarde (${PRETHOLD} %) atteint"
        if save_repository "pré-sauvegarde batterie ${capacity}%"; then
            : >"$STATE_PRE"
        fi
    fi

    if [ "$capacity" -le "$THRESHOLD" ] && [ ! -e "$STATE_DONE" ]; then
        log "Seuil critique (${THRESHOLD} %) atteint"
        save_repository "batterie ${capacity}%" || true

        # Si le chargeur a été rebranché entre-temps, on annule l'extinction.
        battery="$(read_battery)" || battery=""
        capacity="${battery%% *}"
        status="${battery##* }"
        if [ -n "$battery" ] && { [ "$status" != "Discharging" ] || [ "${capacity:-0}" -gt $((THRESHOLD + 2)) ]; }; then
            log "Secteur reconnecté (${capacity} %, ${status}) : extinction annulée"
            return 0
        fi

        : >"$STATE_DONE"
        shutdown_now
        return 0
    fi
    return 0
}

case "${1:---watch}" in
    --check)
        battery="$(read_battery)" || { echo "aucune batterie"; exit 0; }
        echo "batterie=${battery%% *}% statut=${battery##* } seuil=${THRESHOLD}% pré-seuil=${PRETHOLD}%"
        ;;
    --backup-only)
        save_repository "sauvegarde manuelle" || exit 1
        ;;
    --once)
        check_once
        ;;
    --watch)
        log "Surveillance batterie démarrée (pré-seuil ${PRETHOLD} %, extinction ${THRESHOLD} %)"
        while :; do
            check_once
            sleep "$INTERVAL"
        done
        ;;
    *)
        echo "usage: $0 [--check|--backup-only|--once|--watch]" >&2
        exit 2
        ;;
esac
