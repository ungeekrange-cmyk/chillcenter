#!/bin/sh
# Synthetic provider for QA only. No network, hooks, credentials or catalogue reads.
input=$(/bin/cat)
case "$input" in
 *"Recherche seulement, aucun changement autorisé."*) /usr/bin/printf '%s' '{"message":"Recherche synthétique","changes":[],"matched_ids":["video-0000"]}' ;;
 *) /usr/bin/printf '%s' '{"message":"Proposition synthétique, jamais automatique","changes":[{"id":"video-0000","title":"AI QA corrected","rating":5}],"matched_ids":[]}' ;;
esac
