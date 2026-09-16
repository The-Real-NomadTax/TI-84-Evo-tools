# TI-84 Evo OS 7.1.0.4421

The connected calculator reported:

- Product ID: `23-10-28-21EFbid`
- OS package version: `7.1.0.4421`
- BSP version: `7.1.0.4413`
- BL1 version: `1.0.0.260`
- BL2 version: `7.0.0.3386`

TI's live version manifest also identifies `7.1.0.4421` as the current
`ti84Bundle`. The matching signed bundle can be downloaded from:

`https://connectevo.ti.com/ticevo/en/assets/os/TI84Evo_Bundle_7.1.0.4421.84b2`

The installer blobs are not committed here. If you download them for local
research, expected filenames and SHA-256 hashes are:

- `TI84Evo_Bundle_7.1.0.4421.84b2` — `86fe1b026bbea4538511f6b3241b19c77361dfb95259d0105c941a2459953a96`
- `TI84Evo_Package.84pk2` — `c706d8555e41325fbd6f9c03a9fc87a04c6e1d0875031020d1bbc9b6a08584df`

The `.84pk2` file is the OS payload extracted from the `.84b2` bundle. These
are the official install images matching the calculator's running version, not
a raw flash read and therefore not a copy of device-specific storage.
