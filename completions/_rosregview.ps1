
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'rosregview' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'rosregview'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'rosregview' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'Show a one-line summary for a hive file: size, root subkey count')
            [CompletionResult]::new('tree', 'tree', [CompletionResultType]::ParameterValue, 'Recursively print the key tree of a hive file (uses ASCII-only indent)')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List the direct subkeys of a key (default: the root), with subkey and value counts. The optional `KEY_PATH` may be a single segment (`ControlSet001`) or a backslash-separated subpath (`A\B\C`). Empty path == the root')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show the values of a key (default: the root). Per-value output includes the value name, its REG_* type, and the data, decoded according to the type where possible (UTF-16 strings, u32 little- endian dwords, u64 little-endian qwords, ...). Binary data is shown as a hex dump; long strings are truncated with a trailing `…`')
            [CompletionResult]::new('find', 'find', [CompletionResultType]::ParameterValue, 'Search the key hierarchy for keys with names matching `-n` (substring, repeatable, any-of, case-insensitive by default) and/or values whose name+decoded-data contains `-v`. With no filters, this enumerates every key path up to `--max-depth`')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'rosregview;info' {
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Output format (`human` is the default table, `json` is machine-readable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (`human` is the default table, `json` is machine-readable)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rosregview;tree' {
            [CompletionResult]::new('-d', '-d', [CompletionResultType]::ParameterName, 'Maximum recursion depth. 0 = show only the root, 1 = root + direct children, 2 = up to grand-children, ... Default: unlimited')
            [CompletionResult]::new('--depth', '--depth', [CompletionResultType]::ParameterName, 'Maximum recursion depth. 0 = show only the root, 1 = root + direct children, 2 = up to grand-children, ... Default: unlimited')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Output format (`human` is the indented text tree, `json` is machine-readable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (`human` is the indented text tree, `json` is machine-readable)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rosregview;list' {
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Output format (`human` is an aligned table, `json` is machine-readable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (`human` is an aligned table, `json` is machine-readable)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rosregview;show' {
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Output format (`human` is a typed table, `json` is machine-readable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (`human` is a typed table, `json` is machine-readable)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rosregview;find' {
            [CompletionResult]::new('-n', '-n', [CompletionResultType]::ParameterName, 'Substring to match in key names. Repeat to OR multiple patterns')
            [CompletionResult]::new('--name', '--name', [CompletionResultType]::ParameterName, 'Substring to match in key names. Repeat to OR multiple patterns')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Substring to match in value name + decoded data (any value within a matching key whose name/data contains the pattern)')
            [CompletionResult]::new('--value', '--value', [CompletionResultType]::ParameterName, 'Substring to match in value name + decoded data (any value within a matching key whose name/data contains the pattern)')
            [CompletionResult]::new('--max-depth', '--max-depth', [CompletionResultType]::ParameterName, 'Limit recursion depth. 0 = root only, 1 = root + direct children, ... Default: unlimited')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Output format (`human` is a path list, `json` is machine-readable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (`human` is a path list, `json` is machine-readable)')
            [CompletionResult]::new('--case-sensitive', '--case-sensitive', [CompletionResultType]::ParameterName, 'Match case-sensitively. Default: case-insensitive')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rosregview;help' {
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'Show a one-line summary for a hive file: size, root subkey count')
            [CompletionResult]::new('tree', 'tree', [CompletionResultType]::ParameterValue, 'Recursively print the key tree of a hive file (uses ASCII-only indent)')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List the direct subkeys of a key (default: the root), with subkey and value counts. The optional `KEY_PATH` may be a single segment (`ControlSet001`) or a backslash-separated subpath (`A\B\C`). Empty path == the root')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show the values of a key (default: the root). Per-value output includes the value name, its REG_* type, and the data, decoded according to the type where possible (UTF-16 strings, u32 little- endian dwords, u64 little-endian qwords, ...). Binary data is shown as a hex dump; long strings are truncated with a trailing `…`')
            [CompletionResult]::new('find', 'find', [CompletionResultType]::ParameterValue, 'Search the key hierarchy for keys with names matching `-n` (substring, repeatable, any-of, case-insensitive by default) and/or values whose name+decoded-data contains `-v`. With no filters, this enumerates every key path up to `--max-depth`')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'rosregview;help;info' {
            break
        }
        'rosregview;help;tree' {
            break
        }
        'rosregview;help;list' {
            break
        }
        'rosregview;help;show' {
            break
        }
        'rosregview;help;find' {
            break
        }
        'rosregview;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
