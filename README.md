# twitch-live-notification-to-discord-webhook
pretty long name, sends live notifications to discord

## about
http server using shuttle that gets stream.online notifications from the twitch EventSub API and sends a live notification to a discord webhook

## setup
set up a shuttle project and add the following to Secrets.toml:
```
TWITCH_CLIENT_ID
TWITCH_CLIENT_SECRET
TWITCH_EVENT_SECRET
DISCORD_WEBHOOK_URL
PASSWORD
SERVER_URL
```
- TWITCH_CLIENT_ID: client id for your twitch application
- TWITCH_CLIENT_SECRET: client secret of your application
- TWITCH_EVENT_SECRET: secret you use when subscribing to EventSub notifications
- DISCORD_WEBHOOK_URL: url of the discord webhook you want to send notifications to
- PASSWORD: the password you use to sign into the web ui
- SERVER_URL: the url for your shuttle project

## usage
go to your shuttle website and log in using your PASSWORD

use the prompt add the bottom to add subscriptions

if your subscriptions aren't shown click reverify access token

remove subscriptions with the remove button

retry for subscriptions that have been revoked for some reason, but you want back
