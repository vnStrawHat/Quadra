// The shell-integration line typed into the remote shell after it opens (specs/23). It installs
// OSC 7 and OSC 133 hooks and re-enables echo; bash and zsh get full marks, others the one-shot.
module ssh

const shell_integration_bootstrap =
	r"export COLORTERM=truecolor; __oneterm_mark() { printf '\033]133;%s\033\\' " + '"$1"; }; ' +
	r'__oneterm_precmd() { __oneterm_status=$?; if [ -n "${__oneterm_seen-}" ]; then __oneterm_mark "D;$__oneterm_status"; fi; __oneterm_seen=1; ' +
	r"printf '\033]7;file://%s%s\033\\' " +
	r'"${HOSTNAME:-$(hostname)}" "$PWD"; __oneterm_mark A; return $__oneterm_status; }; ' +
	r'__oneterm_preexec() { __oneterm_mark C; }; if [ -n "${ZSH_VERSION-}" ]; then ' +
	r"eval 'precmd_functions=(__oneterm_precmd $precmd_functions); preexec_functions=($preexec_functions __oneterm_preexec)'; " +
	r'PS1="$PS1%{$(__oneterm_mark B)%}"; elif [ -n "${BASH_VERSION-}" ]; then case ";${PROMPT_COMMAND:-};" in *";__oneterm_precmd;"*) ;; *) ' +
	r'PROMPT_COMMAND="__oneterm_precmd${PROMPT_COMMAND:+;$PROMPT_COMMAND}" ;; esac; PS0="$(printf ' +
	r"'\033]133;C\007'" + r')"; ' + r'PS1="$PS1$(printf ' + r"'\001\033]133;B\007\002'" +
	r')"; fi; __oneterm_precmd; __oneterm_seen=; stty echo 2>/dev/null'
