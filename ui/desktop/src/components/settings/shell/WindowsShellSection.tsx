import { useEffect, useState, useCallback } from 'react';
import { useConfig } from '../../ConfigContext';
import { Select } from '../../ui/Select';

const SHELL_OPTIONS = [
  { value: '', label: 'Auto-detect (recommended)' },
  { value: 'bash', label: 'Bash (Git Bash / MSYS2)' },
  { value: 'gitbash', label: 'Git Bash (explicit path)' },
  { value: 'powershell', label: 'Windows PowerShell' },
  { value: 'pwsh', label: 'PowerShell Core (pwsh)' },
  { value: 'cmd', label: 'Command Prompt (cmd.exe)' },
  { value: 'wsl', label: 'WSL (Windows Subsystem for Linux)' },
];

export const WindowsShellSection = () => {
  const [currentShell, setCurrentShell] = useState('');
  const { read, upsert, remove } = useConfig();

  const fetchCurrentShell = useCallback(async () => {
    try {
      const shell = (await read('GOOSE_WINDOWS_SHELL', false)) as string;
      if (shell) {
        setCurrentShell(shell);
      }
    } catch (error) {
      console.error('Error fetching Windows shell setting:', error);
    }
  }, [read]);

  useEffect(() => {
    fetchCurrentShell();
  }, [fetchCurrentShell]);

  const handleShellChange = async (option: { value: string; label: string } | null) => {
    const newShell = option?.value || '';
    try {
      if (newShell) {
        await upsert('GOOSE_WINDOWS_SHELL', newShell, false);
      } else {
        await remove('GOOSE_WINDOWS_SHELL', false);
      }
      setCurrentShell(newShell);
    } catch (error) {
      console.error('Error updating Windows shell:', error);
    }
  };

  const selectedOption = SHELL_OPTIONS.find((opt) => opt.value === currentShell) || SHELL_OPTIONS[0];

  return (
    <div className="space-y-2 px-4 py-3">
      <div className="flex items-center justify-between gap-4">
        <div className="flex-1 min-w-0">
          <h4 className="text-sm font-medium text-text-primary">Shell</h4>
          <p className="text-xs text-text-muted mt-0.5">
            Shell used for command execution. Auto-detect prefers Git Bash when available.
          </p>
        </div>
        <div className="w-64 flex-shrink-0">
          <Select
            value={selectedOption}
            onChange={handleShellChange}
            options={SHELL_OPTIONS}
            isSearchable={false}
          />
        </div>
      </div>
    </div>
  );
};
