# Echo Anticheat exploit disabling Antivirus and Endpoint Detection and Response (EDR)

Write-up of the exploit can be found here: 

## Disabling Antivirus and EDR through the use of EchoDrv

This proof-of-concept, partically, disables EDR and AV through patching the kernel notify routines of PspCreateProcessNotifyRoutine, PspCreateThreadNotifyRoutine and PspLoadImageNotifyRoutine. Aswell as disabling ETW (Event Tracing Windows) for the Windows Threat Provider commonly used by AV/EDR.

## Disclaimer

I am not responsible for what you do with the information and code provided. This is intended for professional or educational purposes only.
