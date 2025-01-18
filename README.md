# Twitch Screenshot Organizer

FFZ gives you an option to take screenshots that is saves in downloads. This basically organises them into a folders with subfolders based on the channel.

## Usage

-   TO build -> `cargo build --release`
-   `twitch-screenshot-organizer <path to downloads>`
-   `--watch` to keep it running and watch for new screenshots ( prob set this up as systemd service )

## Systemd Service

-   Create this file in `/etc/systemd/system/twitch-screenshot-organizer.service`

```
[Unit]
Description=Daemon to run twitch-screenshot-organizer

[Service]
User=<USERNAME>
Group=<USERNAME>
ExecStart=<PATH TO BINARY> <PATH TO DOWNLOADS> --watch
ExecReload=/bin/kill -s HUP $MAINPID
RestartSec=5

[Install]
WantedBy=multi-user.target

```

-   `sudo systemctl enable twitch-screenshot-organizer`
-   `sudo systemctl start twitch-screenshot-organizer`

> LULE ik its an overkill for a simple task but this was a quick project so...
