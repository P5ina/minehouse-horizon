# Plasmo Voice for Horizon

A native [Horizon](https://github.com/horizon-rust/horizon) mod (Minecraft 26.3) that speaks the
[Plasmo Voice](https://github.com/plasmoapp/plasmo-voice) 2.x protocol, so a Horizon player talks with
Java players who use the Plasmo Voice mod on the same server (MineHouse runs Plasmo Voice 2.1.17).

- **Talking:** push to talk (Caps Lock) or voice activation, on the server's proximity activation at its
  default distance. Voices come from where players stand: quieter with distance, panned left or right.
- **Menu (V):** status, microphone and voice chat on/off, mode, volumes and the players with voice chat.
- **`/vc`:** `status`, `mute`/`unmute`, `off`/`on`, `mode ptt|voice`, `volume`, `gain`, `threshold`.

How it connects: it registers `plasmo:voice/v2`, sends its RSA public key (`PlayerInfoPacket`), gets
the UDP address and secret (`ConnectionPacket`), pings the voice server, and opens the server's AES key
from `ConfigPacket` with its private key. Voice is Opus (pure-Rust rusty-opus: nothing to install) in
`AES/CBC/PKCS5Padding` frames over UDP.

## Credits

Built on [Horizon Voice Chat](https://github.com/Nirokx/horizon-voice-chat) by Nirokx (MIT): the audio
devices, the mixer, the menu, the HUD and the hook glue. The Plasmo protocol, encryption and connection
are new. Not affiliated with Plasmo Voice or its authors; none of Plasmo Voice's code is included.

License: MIT.
