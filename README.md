# Follower

## What is this and why does it exist?

This exists as like a mini project to help me keep track of how i know people and archive certain things, without being really intrusive.

It only has access to what a bot can see from the public view (username, display name, avatar, banner, badges, account creation date). Nothing is read in the background, the bot never watches your messages or servers. It only does something when **you** run a command or use one of the right click menus.

It is a **self hosted** bot, not a service. You run your own copy, with your own database, and you are the only one who can use it.

I personally see it as a way to just help me be a bit more organised. And its also just because i wanted to make something.

## What does it store (and what doesnt it)?

**Stored in the database:**

- The people you add to your list: their user ID, relation (friend, unfriended, blocked...), your notes, where you know them from, their birthday if you enter one, and tags
- Profile snapshots: every time a profile changes (new avatar, new username...) a snapshot is saved so you can see the history. These are only the public profile fields and **links** to the avatar/banner, not the images themselves
- A log of every time you changed someones relation
- Archived messages: **only the ones you manually save** with "Save to archive". It stores the text, author ID/name/avatar link, the message link, and the attachments (see "Where is the data at?")

**Not stored:**

- Anything you didnt explicitly ask it to save
- Messages from servers/DMs it just happens to be in
- Older avatars/banners as actual images (only the latest one gets fetched, the snapshots just keep the links)

The person that operates the bot is responsible for the data as they are the only one with access to it. Anyone external that might've installed it cannot run any commands, every single command and menu is locked to the `OWNER_ID` you set.

## How do i set it up?

You need **Python 3.10 or newer** and a mysql or mariadb server (there are plenty of guides online for that, i am not going to explain how to set one up in here).

1. Grab the code from the [Bot folder](bot/) and install the requirements with `pip install -r requirements.txt` (i recommend you use a Virtual Environment, also known as venv, you can create one using `python3 -m venv env` and activate it on linux by doing `source env/bin/activate`)
2. Create an empty database and a user for it, for example `CREATE DATABASE follower CHARACTER SET utf8mb4;`. The bot creates all the tables by itself on first start, but it will **not** create the database itself
3. Copy `.env.example` to `.env` and fill out what is asked (see below)
4. Set up the bot on Discord (see the next section)
5. Run it with `python bot.py`

### The .env file

- `DISCORD_TOKEN`: the bot token from the [Discord developer portal](https://discord.dev/)
- `OWNER_ID`: your Discord user ID. Enable developer mode (search online how to), then right click yourself and click Copy User ID
- `DB_HOST`, `DB_PORT`, `DB_USER`, `DB_PASSWORD`, `DB_NAME`: your mysql/mariadb info
- `TIMEZONE`: used for birthdays, like `America/Toronto`
- `MAX_ATTACHMENT_MB`: biggest attachment that gets saved (more on that below)
- `API_TOKEN`: only needed if you want to use the apps. If you leave it empty the api just doesnt start. Make one with `python -c "import secrets;print(secrets.token_urlsafe(32))"`
- `API_PORT`: port of the api (default 8080)
- `API_HOST`: optional, defaults to `0.0.0.0` (reachable from everywhere). If you use a tunnel on the same machine (see below) set it to `127.0.0.1`

**Careful:** dont put comments on the same line as a value, `.env` doesnt support that and it will break things. Put them on their own line starting with `#`.

## How do i add it to Discord?

This is a **User Installed bot**, not a server bot, so this part matters:

1. Create an application on the [Discord developer portal](https://discord.dev/) and grab the bot token in the Bot tab
2. Go to **Installation**, enable **User Install** and use the **Install Link** it gives you (Discord provided link is fine). Open that link and add it to your own account
3. Start the bot. The commands can take a little bit to show up the first time

Thats it, the commands work in servers, DMs and group DMs, even ones the bot is not in.

## What discord intents are needed?

None. The bot uses the default intents only, no privileged intents (message content, members, presence) need to be turned on. When you save a message to the archive, Discord hands the bot that message directly because **you** used a menu on it.

If you want to run this publicly (i do not recommend that, and you should only be using it for personal use because this has NOT been coded for that kind of use) you should really look at what that means first, see the Q&A.

## What can it do?

Right click a user or a message, then go to **Apps**:

- **Manage person**: change relation, tags, notes, birthday, or remove them from your list
- **Lookup user**: shows their profile and what changed since last time you checked
- **Lookup mentions**: looks up everyone mentioned in a message
- **Save to archive**: saves that message (and its attachments)

Slash commands:

- `/lookup`: look up one or more people (pick users and/or paste IDs or @mentions), can save them directly
- `/fetchall`: refresh everyone on your list and browse them
- `/people`: list the people you track, filter by relation and/or tag
- `/status`, `/note`, `/met`: set relation, notes, and where you know someone from
- `/history`: relation + profile change history of someone
- `/upcoming`: upcoming birthdays
- `/tag add`, `/tag remove`, `/tag list`
- `/archive search`, `/archive random`, `/archive delete`

## Using the apps and the api

If you intend to use the api with the apps ive provided (macOS is not included because i do not own a Mac and i do not have the time and patience to do a Hackintosh) you must set an `API_TOKEN`, and enter that same token in the app.

The api only uses a token, so **do not expose it over plain http**. If you want a quick and painless way to give it an external url accessible from anywhere with SSL, use Cloudflare Tunnels with your own domain.

If you intend to use the iOS/iPadOS version and arent a developer, you will need to sideload it every 7 days **UNLESS** you got a jailbroken device. The iOS app lets you use a plain http url out of the box (handy with something like Tailscale), but http sends your token unencrypted, so only do that on a private network.

You will also notice Windows, Android and Linux builds, i have made them to feel as native as possible to the OS you are using.

The apps requirements are Windows 10/11, Android 10+, Linux (just linux because who needs a specific version, all of them are equal, right?), iOS/iPadOS 16 or higher.

## How can i build them?

For Windows i recommend heavily to use the csproj that is bundled with it and to just use Visual Studio 2022 or 2026 (i used 2026, it works)

For Linux make sure you are in the directory and just run `cargo build -p follower --release`

For Android, you can either use Android Studio, or when in the android directory, you can run `./gradlew :app:assembleRelease` or `./gradlew :app:assembleDebug` (depending on your needs, but in general release will do the job)

For iOS, i havent used a Mac but i have used a tool called xtool allowing me to build this app, just run `xtool dev build --ipa` to generate an ipa

## Where is the data at?

All data, and when i mean all, i mean ALL of it, is stored in the database. If you need to quickly delete everything, just delete the database itself fully and everything goes *poof* gone.

If you only want to delete specific things:

- **Archived messages:** `/archive delete` (or Select the archive entry in the app and click delete). Deleting a message also deletes its saved files
- **People:** in the apps Select someone and click delete, or on Discord right click the person, go to Apps, Manage person and click Remove from list. **Heads up:** this removes them from your list and their tags, but the profile snapshots and relation history stay in the database until you delete the database itself
- **Tags** auto delete once they are no longer in use by anyone

Attachments that are smaller than the size set in the `.env` file are stored directly in the database (in their own table, linked to the archived message, and compressed when that actually saves space). Bigger ones are not saved, only their name, size and the Discord link are kept, and those links can expire. If you raise `MAX_ATTACHMENT_MB` a lot, make sure your database `max_allowed_packet` setting is big enough too or saving will fail.

You can only get the latest image from the account that you are fetching because the server does not store older ones, it updates if you ask it to fetch all, when you open the user with a lookup, or on the apps.

## Licenses

You can find all the mentions of licenses in the [Licensing file](LICENSING.md)

## Q&A

Q. Does my friend get notified when i do any actions at all?
A. No, they do not. You are responsible for what you store and/or put about them.

Q. Can i turn this into a Public service?
A. I mean, if you really want, but as ive mentioned above, i dont recommend it. It is built as a personal User Installed bot, not for that kind of use. If you do, you are responsible for all the data deletion requirements per your local law and in all other countries that this bot can reach.

Q. Will you keep this updated?
A. Possibly, i am not sure honestly, i built this out of just pure boredom and because i need to start getting organised (and maybe just archive a few things)

Q. Does this break Discord's TOS?
A. To my knowledge, no it doesnt. **BUT** if you do edit it to do more than what it should be intended to do and/or breaks their TOS, then yes it does. **Edit at your own risk.** Since you host it yourself, you are also the one responsible for following Discord's rules and your local privacy laws for whatever you store.

Q. It doesnt start / the commands dont show up, what do i do?
A. Check that your `.env` has no comments on the same line as a value, that the database exists and the user can reach it, and that you enabled **User Install** in the developer portal. You can always open an issue to let me know or do a PR request.

## Contact

You can i guess contact me thru my [Discord](https://discord.com/users/794456700416425995) but i doubt i have friend invites open.
No i will not share an email address for this.
