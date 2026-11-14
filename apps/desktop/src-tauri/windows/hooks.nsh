; Allow LAN sync on private networks only, so Windows doesn't block the phone.
!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog 'netsh advfirewall firewall delete rule name="Coachs Playbook Sync"'
  nsExec::ExecToLog 'netsh advfirewall firewall add rule name="Coachs Playbook Sync" dir=in action=allow protocol=TCP localport=47820 profile=private'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog 'netsh advfirewall firewall delete rule name="Coachs Playbook Sync"'
!macroend
