# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_rosregview_global_optspecs
    string join \n h/help V/version
end

function __fish_rosregview_needs_command
    # Figure out if the current invocation already has a command.
    set -l cmd (commandline -opc)
    set -e cmd[1]
    argparse -s (__fish_rosregview_global_optspecs) -- $cmd 2>/dev/null
    or return
    if set -q argv[1]
        # Also print the command, so this can be used to figure out what it is.
        echo $argv[1]
        return 1
    end
    return 0
end

function __fish_rosregview_using_subcommand
    set -l cmd (__fish_rosregview_needs_command)
    test -z "$cmd"
    and return 1
    contains -- $cmd[1] $argv
end

complete -c rosregview -n "__fish_rosregview_needs_command" -s h -l help -d 'Print help'
complete -c rosregview -n "__fish_rosregview_needs_command" -s V -l version -d 'Print version'
complete -c rosregview -n "__fish_rosregview_needs_command" -f -a "info" -d 'Show a one-line summary for a hive file: size, root subkey count'
complete -c rosregview -n "__fish_rosregview_needs_command" -f -a "tree" -d 'Recursively print the key tree of a hive file (uses ASCII-only indent)'
complete -c rosregview -n "__fish_rosregview_needs_command" -f -a "list" -d 'List the direct subkeys of a key (default: the root), with subkey and value counts. The optional `KEY_PATH` may be a single segment (`ControlSet001`) or a backslash-separated subpath (`A\\B\\C`). Empty path == the root'
complete -c rosregview -n "__fish_rosregview_needs_command" -f -a "show" -d 'Show the values of a key (default: the root). Per-value output includes the value name, its REG_* type, and the data, decoded according to the type where possible (UTF-16 strings, u32 little- endian dwords, u64 little-endian qwords, ...). Binary data is shown as a hex dump; long strings are truncated with a trailing `…`'
complete -c rosregview -n "__fish_rosregview_needs_command" -f -a "find" -d 'Search the key hierarchy for keys with names matching `-n` (substring, repeatable, any-of, case-insensitive by default) and/or values whose name+decoded-data contains `-v`. With no filters, this enumerates every key path up to `--max-depth`'
complete -c rosregview -n "__fish_rosregview_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c rosregview -n "__fish_rosregview_using_subcommand info" -s f -l format -d 'Output format (`human` is the default table, `json` is machine-readable)' -r -f -a "human\t''
json\t''"
complete -c rosregview -n "__fish_rosregview_using_subcommand info" -s h -l help -d 'Print help'
complete -c rosregview -n "__fish_rosregview_using_subcommand tree" -s d -l depth -d 'Maximum recursion depth. 0 = show only the root, 1 = root + direct children, 2 = up to grand-children, ... Default: unlimited' -r
complete -c rosregview -n "__fish_rosregview_using_subcommand tree" -s f -l format -d 'Output format (`human` is the indented text tree, `json` is machine-readable)' -r -f -a "human\t''
json\t''"
complete -c rosregview -n "__fish_rosregview_using_subcommand tree" -s h -l help -d 'Print help'
complete -c rosregview -n "__fish_rosregview_using_subcommand list" -s f -l format -d 'Output format (`human` is an aligned table, `json` is machine-readable)' -r -f -a "human\t''
json\t''"
complete -c rosregview -n "__fish_rosregview_using_subcommand list" -s h -l help -d 'Print help'
complete -c rosregview -n "__fish_rosregview_using_subcommand show" -s f -l format -d 'Output format (`human` is a typed table, `json` is machine-readable)' -r -f -a "human\t''
json\t''"
complete -c rosregview -n "__fish_rosregview_using_subcommand show" -s h -l help -d 'Print help'
complete -c rosregview -n "__fish_rosregview_using_subcommand find" -s n -l name -d 'Substring to match in key names. Repeat to OR multiple patterns' -r
complete -c rosregview -n "__fish_rosregview_using_subcommand find" -s v -l value -d 'Substring to match in value name + decoded data (any value within a matching key whose name/data contains the pattern)' -r
complete -c rosregview -n "__fish_rosregview_using_subcommand find" -l max-depth -d 'Limit recursion depth. 0 = root only, 1 = root + direct children, ... Default: unlimited' -r
complete -c rosregview -n "__fish_rosregview_using_subcommand find" -s f -l format -d 'Output format (`human` is a path list, `json` is machine-readable)' -r -f -a "human\t''
json\t''"
complete -c rosregview -n "__fish_rosregview_using_subcommand find" -l case-sensitive -d 'Match case-sensitively. Default: case-insensitive'
complete -c rosregview -n "__fish_rosregview_using_subcommand find" -s h -l help -d 'Print help'
complete -c rosregview -n "__fish_rosregview_using_subcommand help; and not __fish_seen_subcommand_from info tree list show find help" -f -a "info" -d 'Show a one-line summary for a hive file: size, root subkey count'
complete -c rosregview -n "__fish_rosregview_using_subcommand help; and not __fish_seen_subcommand_from info tree list show find help" -f -a "tree" -d 'Recursively print the key tree of a hive file (uses ASCII-only indent)'
complete -c rosregview -n "__fish_rosregview_using_subcommand help; and not __fish_seen_subcommand_from info tree list show find help" -f -a "list" -d 'List the direct subkeys of a key (default: the root), with subkey and value counts. The optional `KEY_PATH` may be a single segment (`ControlSet001`) or a backslash-separated subpath (`A\\B\\C`). Empty path == the root'
complete -c rosregview -n "__fish_rosregview_using_subcommand help; and not __fish_seen_subcommand_from info tree list show find help" -f -a "show" -d 'Show the values of a key (default: the root). Per-value output includes the value name, its REG_* type, and the data, decoded according to the type where possible (UTF-16 strings, u32 little- endian dwords, u64 little-endian qwords, ...). Binary data is shown as a hex dump; long strings are truncated with a trailing `…`'
complete -c rosregview -n "__fish_rosregview_using_subcommand help; and not __fish_seen_subcommand_from info tree list show find help" -f -a "find" -d 'Search the key hierarchy for keys with names matching `-n` (substring, repeatable, any-of, case-insensitive by default) and/or values whose name+decoded-data contains `-v`. With no filters, this enumerates every key path up to `--max-depth`'
complete -c rosregview -n "__fish_rosregview_using_subcommand help; and not __fish_seen_subcommand_from info tree list show find help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
