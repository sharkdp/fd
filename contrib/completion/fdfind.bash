# Source fd completions
source /usr/share/bash-completion/completions/fd

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
  complete -F _fd -o nosort -o bashdefault -o default fdfind
else
  complete -F _fd -o bashdefault -o default fdfind
fi
