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
```
- TWITCH_CLIENT_ID: client id for your twitch application
- TWITCH_CLIENT_SECRET: client secret of your application
- TWITCH_EVENT_SECRET: secret you use when subscribing to EventSub notifications
- DISCORD_WEBHOOK_URL: url of the discord webhook you want to send notifications to

note: not all off these are currently neccesary for the application and might be removed later

### the stuff you have to do yourself right now

this server currently only relays the messages to the discord webhook and has no features to manage subscriptions
you will have to subscribe to events yourself using the twitch api

https://dev.twitch.tv/docs/eventsub/eventsub-subscription-types/#streamonline

the callback is the url of the shuttle project
secret is the same secret as in your TWITCH_EVENT_SECRET
broadcaster_user_id is the user id of the broadcaster for which you want to get notifications

you will also need your application's client id and app access token in the headers to subscribe.

