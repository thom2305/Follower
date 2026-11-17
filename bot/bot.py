# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright 2026 Thom (thom2305)
#
# This program is free software: you can redistribute it and/or modify it under
# the terms of the GNU Affero General Public License as published by the Free
# Software Foundation, either version 3 of the License, or (at your option) any
# later version. If you run a modified version of this program for other people
# over a network, you must offer them its source code (AGPL section 13).
# See the LICENSE file in this folder.

import sys
import asyncio
import calendar
import datetime as dt
import io
import json
import os
import re
import zlib
from zoneinfo import ZoneInfo

import aiomysql
import discord
from discord import app_commands
from dotenv import load_dotenv

load_dotenv()

TOKEN = os.environ["DISCORD_TOKEN"]
OWNER_ID = int(os.environ["OWNER_ID"])
TZ = ZoneInfo(os.environ.get("TIMEZONE", "America/Toronto"))
MAX_ATTACH = int(os.environ.get("MAX_ATTACHMENT_MB", "8")) * 1024 * 1024

INCOMPRESSIBLE = (
    "image/jpeg", "image/png", "image/webp", "image/gif", "video/", "audio/",
    "application/zip", "application/gzip", "application/x-7z", "application/x-rar",
)

STATUSES = {
    "friend": "Friend",
    "unfriended": "I unfriended them",
    "unfriended_me": "They unfriended me",
    "blocked": "I blocked them",
    "blocked_me": "They blocked me",
    "acquaintance": "Acquaintance",
    "other": "Other",
}
STATUS_CHOICES = [app_commands.Choice(name=v, value=k) for k, v in STATUSES.items()]

SCHEMA = [
    """CREATE TABLE IF NOT EXISTS people (
        user_id BIGINT UNSIGNED PRIMARY KEY,
        relation VARCHAR(32) NOT NULL DEFAULT 'acquaintance',
        notes TEXT NULL,
        met_context VARCHAR(64) NULL,
        birthday_month TINYINT UNSIGNED NULL,
        birthday_day TINYINT UNSIGNED NULL,
        birthday_year SMALLINT UNSIGNED NULL,
        first_seen DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
    )""",
    """CREATE TABLE IF NOT EXISTS people_tags (
        user_id BIGINT UNSIGNED NOT NULL,
        tag VARCHAR(32) NOT NULL,
        PRIMARY KEY (user_id, tag),
        INDEX (tag)
    )""",
    """CREATE TABLE IF NOT EXISTS snapshots (
        id BIGINT AUTO_INCREMENT PRIMARY KEY,
        user_id BIGINT UNSIGNED NOT NULL,
        taken_at DATETIME NOT NULL,
        data LONGTEXT NOT NULL,
        INDEX (user_id)
    )""",
    """CREATE TABLE IF NOT EXISTS archive (
        message_id BIGINT UNSIGNED PRIMARY KEY,
        author_id BIGINT UNSIGNED NOT NULL,
        author_name VARCHAR(100) NOT NULL,
        author_avatar VARCHAR(512) NULL,
        content TEXT NULL,
        jump_url VARCHAR(255) NULL,
        channel_id BIGINT UNSIGNED NULL,
        guild_id BIGINT UNSIGNED NULL,
        message_ts DATETIME NOT NULL,
        saved_at DATETIME NOT NULL,
        INDEX (author_id),
        INDEX (saved_at)
    ) ENGINE=InnoDB""",
    """CREATE TABLE IF NOT EXISTS archive_files (
        id BIGINT AUTO_INCREMENT PRIMARY KEY,
        message_id BIGINT UNSIGNED NOT NULL,
        filename VARCHAR(255) NOT NULL,
        content_type VARCHAR(128) NULL,
        size BIGINT UNSIGNED NOT NULL,
        url TEXT NULL,
        compressed TINYINT(1) NOT NULL DEFAULT 0,
        stored_size BIGINT UNSIGNED NOT NULL DEFAULT 0,
        data LONGBLOB NULL,
        INDEX (message_id),
        FOREIGN KEY (message_id) REFERENCES archive(message_id) ON DELETE CASCADE
    ) ENGINE=InnoDB""",
    """CREATE TABLE IF NOT EXISTS status_log (
        id BIGINT AUTO_INCREMENT PRIMARY KEY,
        user_id BIGINT UNSIGNED NOT NULL,
        status VARCHAR(32) NOT NULL,
        at DATETIME NOT NULL,
        INDEX (user_id)
    )""",
]


def now():
    return dt.datetime.now(dt.timezone.utc).replace(tzinfo=None)


def aware(d: dt.datetime) -> dt.datetime:
    return d if d.tzinfo else d.replace(tzinfo=dt.timezone.utc)


def human(n: float) -> str:
    for unit in ("B", "KB", "MB", "GB"):
        if n < 1024 or unit == "GB":
            return f"{n:.0f} {unit}" if unit == "B" else f"{n:.1f} {unit}"
        n /= 1024


class FollowerBot(discord.Client):
    def __init__(self):
        super().__init__(intents=discord.Intents.default())
        self.tree = app_commands.CommandTree(
            self,
            allowed_installs=app_commands.AppInstallationType(guild=False, user=True),
            allowed_contexts=app_commands.AppCommandContext(
                guild=True, dm_channel=True, private_channel=True
            ),
        )
        self.pool: aiomysql.Pool | None = None

    async def setup_hook(self):
        self.pool = await aiomysql.create_pool(
            host=os.environ.get("DB_HOST", "localhost"),
            port=int(os.environ.get("DB_PORT", 3306)),
            user=os.environ["DB_USER"],
            password=os.environ["DB_PASSWORD"],
            db=os.environ["DB_NAME"],
            autocommit=True,
            charset="utf8mb4",
        )
        async with self.pool.acquire() as c, c.cursor() as cur:
            for stmt in SCHEMA:
                await cur.execute(stmt)
        await self.tree.sync()
        import api
        await api.start(self, sys.modules[__name__])

    async def q(self, sql, *args, one=False, write=False):
        async with self.pool.acquire() as c, c.cursor() as cur:
            await cur.execute(sql, args)
            if write:
                return cur.rowcount
            return await cur.fetchone() if one else await cur.fetchall()


bot = FollowerBot()


@bot.tree.error
async def on_error(i: discord.Interaction, err: app_commands.AppCommandError):
    msg = "Not yours." if isinstance(err, app_commands.CheckFailure) else f"Error: {err}"
    if i.response.is_done():
        await i.followup.send(msg, ephemeral=True)
    else:
        await i.response.send_message(msg, ephemeral=True)


def owner_only():
    async def pred(i: discord.Interaction):
        return i.user.id == OWNER_ID
    return app_commands.check(pred)


def profile_of(u: discord.User) -> dict:
    deco = getattr(u, "avatar_decoration", None)
    return {
        "username": u.name,
        "global_name": u.global_name,
        "avatar": u.avatar.url if u.avatar else None,
        "banner": u.banner.url if u.banner else None,
        "accent_color": u.accent_color.value if u.accent_color else None,
        "avatar_decoration": deco.url if deco else None,
        "bot": u.bot,
        "badges": sorted(n for n, v in u.public_flags if v),
        "created_at": u.created_at.isoformat(),
    }


def diff(old: dict, new: dict) -> dict:
    return {k: (old.get(k), new.get(k)) for k in new if old.get(k) != new.get(k)}


async def last_profile(user_id: int):
    row = await bot.q("SELECT data FROM snapshots WHERE user_id=%s ORDER BY id DESC LIMIT 1", user_id, one=True)
    return json.loads(row[0]) if row else None


async def take_snapshot(user_id: int):
    try:
        u = await bot.fetch_user(user_id)
    except discord.NotFound:
        return None, {}, False
    new = profile_of(u)
    old = await last_profile(user_id)
    changes = diff(old, new) if old else {}
    if old is None or changes:
        await bot.q(
            "INSERT INTO snapshots (user_id, taken_at, data) VALUES (%s,%s,%s)",
            user_id, now(), json.dumps(new), write=True,
        )
    return new, changes, old is None


PERSON_COLS = "relation, notes, met_context, first_seen, birthday_month, birthday_day, birthday_year"


async def get_person(user_id: int):
    row = await bot.q(f"SELECT {PERSON_COLS} FROM people WHERE user_id=%s", user_id, one=True)
    return dict(zip(PERSON_COLS.split(", "), row)) if row else None


async def get_tags(user_id: int) -> list[str]:
    rows = await bot.q("SELECT tag FROM people_tags WHERE user_id=%s ORDER BY tag", user_id)
    return [r[0] for r in rows]


async def ensure_person(user_id: int, relation: str = "acquaintance"):
    await bot.q("INSERT IGNORE INTO people (user_id, relation) VALUES (%s,%s)", user_id, relation, write=True)


async def set_relation(user_id: int, relation: str):
    await ensure_person(user_id, relation)
    await bot.q("UPDATE people SET relation=%s WHERE user_id=%s", relation, user_id, write=True)
    await bot.q("INSERT INTO status_log (user_id, status, at) VALUES (%s,%s,%s)", user_id, relation, now(), write=True)


def norm_tag(t: str) -> str:
    return re.sub(r"\s+", "-", t.strip().lower())[:32]


async def tag_autocomplete(i: discord.Interaction, current: str):
    if i.user.id != OWNER_ID:
        return []
    cur = norm_tag(current)
    esc = cur.replace("\\", "\\\\").replace("%", "\\%").replace("_", "\\_")
    rows = await bot.q(
        "SELECT tag, COUNT(*) c FROM people_tags WHERE tag LIKE %s GROUP BY tag ORDER BY c DESC LIMIT 25",
        f"%{esc}%",
    )
    choices = [app_commands.Choice(name=f"{t} ({c})", value=t) for t, c in rows]
    if cur and all(t != cur for t, _ in rows):
        choices.insert(0, app_commands.Choice(name=f"+ new tag: {cur}", value=cur))
    return choices[:25]

def parse_birthday(text: str):
    t = text.strip()
    if not t:
        return None, None, None
    m = re.fullmatch(r"(?:(\d{4})-)?(\d{1,2})-(\d{1,2})", t)
    if not m:
        raise ValueError("bad format")
    y = int(m.group(1)) if m.group(1) else None
    mo, d = int(m.group(2)), int(m.group(3))
    dt.date(y or 2000, mo, d)
    return y, mo, d


def next_birthday(mo: int, d: int, today: dt.date) -> dt.date:
    for yr in (today.year, today.year + 1):
        try:
            c = dt.date(yr, mo, d)
        except ValueError:
            c = dt.date(yr, 2, 28)
        if c >= today:
            return c


def fmt_birthday(mo: int, d: int, y: int | None) -> str:
    today = dt.datetime.now(TZ).date()
    nxt = next_birthday(mo, d, today)
    days = (nxt - today).days
    when = "today" if days == 0 else f"in {days} day{'s' if days != 1 else ''}"
    age = f" · turns {nxt.year - y}" if y else ""
    return f"{calendar.month_name[mo]} {d}{age} ({when})"


def build_embed(user_id: int, p: dict | None, changes: dict, person: dict | None, tags: list[str]) -> discord.Embed:
    if p is None:
        e = discord.Embed(title=f"{user_id}", description="Account not found (deleted?)", color=0xE74C3C)
    else:
        e = discord.Embed(
            title=p["global_name"] or p["username"],
            description=f"@{p['username']} · `{user_id}`",
            color=p["accent_color"] or 0x5865F2,
        )
        if p["avatar"]:
            e.set_thumbnail(url=p["avatar"])
        if p["banner"]:
            e.set_image(url=p["banner"])
        created = dt.datetime.fromisoformat(p["created_at"])
        e.add_field(name="Created", value=discord.utils.format_dt(created, "F"), inline=False)
        e.add_field(name="Badges", value=", ".join(p["badges"]) or "none")
        e.add_field(name="Bot", value=str(p["bot"]))
    if person:
        e.add_field(name="Relation", value=STATUSES.get(person["relation"], person["relation"]))
        e.add_field(name="Tracked since", value=discord.utils.format_dt(aware(person["first_seen"]), "R"))
        if person["birthday_month"]:
            e.add_field(
                name="Birthday",
                value=fmt_birthday(person["birthday_month"], person["birthday_day"], person["birthday_year"]),
                inline=False,
            )
        if person["met_context"]:
            e.add_field(name="Know them from", value=person["met_context"], inline=False)
        if person["notes"]:
            e.add_field(name="Notes", value=person["notes"][:1000], inline=False)
    else:
        e.add_field(name="Your list", value="Not tracked yet", inline=False)
    if tags:
        e.add_field(name="Tags", value=", ".join(tags), inline=False)
    if changes:
        lines = [f"**{k}**: `{a}` → `{b}`" for k, (a, b) in changes.items()]
        e.add_field(name="Changed since last check", value="\n".join(lines)[:1000], inline=False)
    return e


async def do_lookup(i: discord.Interaction, user_id: int):
    await i.response.defer(ephemeral=True)
    p, changes, _ = await take_snapshot(user_id)
    await i.followup.send(embed=build_embed(user_id, p, changes, await get_person(user_id), await get_tags(user_id)))


async def manage_payload(user_id: int, origin: discord.Interaction, fresh: bool = False):
    if fresh:
        p, changes, _ = await take_snapshot(user_id)
    else:
        p, changes = await last_profile(user_id), {}
    person = await get_person(user_id)
    my_tags = await get_tags(user_id)
    top = await bot.q("SELECT tag, COUNT(*) FROM people_tags GROUP BY tag ORDER BY COUNT(*) DESC, tag LIMIT 25")
    embed = build_embed(user_id, p, changes, person, my_tags)
    view = ManageView(user_id, origin, person["relation"] if person else None, my_tags, [t for t, _ in top])
    return embed, view


class DetailsModal(discord.ui.Modal, title="Person details"):
    def __init__(self, parent: "ManageView", person: dict | None):
        super().__init__()
        self.parent = parent
        person = person or {}
        bd = ""
        if person.get("birthday_month"):
            bd = (f"{person['birthday_year']}-" if person.get("birthday_year") else "")
            bd += f"{person['birthday_month']:02d}-{person['birthday_day']:02d}"
        self.notes = discord.ui.TextInput(
            label="Notes", style=discord.TextStyle.paragraph, required=False,
            max_length=1000, default=person.get("notes") or None,
        )
        self.met = discord.ui.TextInput(
            label="Where you know them from", required=False, max_length=60,
            default=person.get("met_context") or None,
        )
        self.birthday = discord.ui.TextInput(
            label="Birthday (YYYY-MM-DD or MM-DD)", required=False, max_length=10, default=bd or None,
        )
        self.new_tags = discord.ui.TextInput(
            label="New tags (comma-separated)", required=False, max_length=200,
            placeholder="school, minecraft",
        )
        for item in (self.notes, self.met, self.birthday, self.new_tags):
            self.add_item(item)

    async def on_submit(self, i: discord.Interaction):
        try:
            y, mo, d = parse_birthday(self.birthday.value)
        except ValueError:
            return await i.response.send_message(
                "Birthday must look like `1999-03-05` or `03-05`. Nothing was saved.", ephemeral=True
            )
        uid = self.parent.user_id
        await ensure_person(uid)
        await bot.q(
            "UPDATE people SET notes=%s, met_context=%s, birthday_month=%s, birthday_day=%s, birthday_year=%s "
            "WHERE user_id=%s",
            self.notes.value.strip() or None, self.met.value.strip() or None, mo, d, y, uid, write=True,
        )
        for t in {norm_tag(x) for x in self.new_tags.value.split(",")}:
            if t:
                await bot.q("INSERT IGNORE INTO people_tags (user_id, tag) VALUES (%s,%s)", uid, t, write=True)
        await self.parent.refresh(i)


class ManageView(discord.ui.View):
    def __init__(self, user_id: int, origin: discord.Interaction, relation, my_tags: list[str], popular: list[str]):
        super().__init__(timeout=300)
        self.user_id = user_id
        self.origin = origin
        self.my_tags = set(my_tags)

        self.rel_select = discord.ui.Select(
            placeholder="Relation…", row=0,
            options=[discord.SelectOption(label=lbl, value=k, default=(k == relation)) for k, lbl in STATUSES.items()],
        )
        self.rel_select.callback = self.on_relation
        self.add_item(self.rel_select)

        self.shown = (list(my_tags) + [t for t in popular if t not in self.my_tags])[:25]
        self.tag_select = None
        if self.shown:
            self.tag_select = discord.ui.Select(
                placeholder="Tags…", row=1, min_values=0, max_values=len(self.shown),
                options=[discord.SelectOption(label=t, value=t, default=t in self.my_tags) for t in self.shown],
            )
            self.tag_select.callback = self.on_tags
            self.add_item(self.tag_select)

    async def interaction_check(self, i: discord.Interaction) -> bool:
        return i.user.id == OWNER_ID

    async def on_timeout(self):
        try:
            await self.origin.edit_original_response(view=None)
        except discord.HTTPException:
            pass

    async def refresh(self, i: discord.Interaction):
        embed, view = await manage_payload(self.user_id, self.origin)
        self.stop()
        await i.response.edit_message(embed=embed, view=view)

    async def on_relation(self, i: discord.Interaction):
        await set_relation(self.user_id, self.rel_select.values[0])
        await self.refresh(i)

    async def on_tags(self, i: discord.Interaction):
        hidden = self.my_tags - set(self.shown)
        wanted = set(self.tag_select.values) | hidden
        await ensure_person(self.user_id)
        for t in wanted - self.my_tags:
            await bot.q("INSERT IGNORE INTO people_tags (user_id, tag) VALUES (%s,%s)", self.user_id, t, write=True)
        for t in self.my_tags - wanted:
            await bot.q("DELETE FROM people_tags WHERE user_id=%s AND tag=%s", self.user_id, t, write=True)
        await self.refresh(i)

    @discord.ui.button(label="Edit details", style=discord.ButtonStyle.primary, row=2)
    async def edit_btn(self, i: discord.Interaction, button: discord.ui.Button):
        await i.response.send_modal(DetailsModal(self, await get_person(self.user_id)))

    @discord.ui.button(label="Remove from list", style=discord.ButtonStyle.danger, row=2)
    async def remove_btn(self, i: discord.Interaction, button: discord.ui.Button):
        await bot.q("DELETE FROM people WHERE user_id=%s", self.user_id, write=True)
        await bot.q("DELETE FROM people_tags WHERE user_id=%s", self.user_id, write=True)
        self.stop()
        await i.response.edit_message(
            content=f"Removed <@{self.user_id}> from your list. (Profile snapshots are kept.)", embed=None, view=None
        )


@bot.tree.context_menu(name="Manage person")
@owner_only()
async def manage_menu(i: discord.Interaction, user: discord.User):
    await i.response.defer(ephemeral=True)
    embed, view = await manage_payload(user.id, i, fresh=True)
    await i.followup.send(embed=embed, view=view, ephemeral=True)


@bot.tree.context_menu(name="Lookup user")
@owner_only()
async def lookup_menu(i: discord.Interaction, user: discord.User):
    await do_lookup(i, user.id)


def parse_ids(text: str | None) -> list[int]:
    return [int(x) for x in re.findall(r"\d{15,20}", text or "")]


async def render_entry(uid: int, p: dict | None, changes: dict) -> discord.Embed:
    return build_embed(uid, p, changes, await get_person(uid), await get_tags(uid))


class ResultPager(discord.ui.View):
    def __init__(self, embeds: list[discord.Embed], origin: discord.Interaction, entries=None):
        super().__init__(timeout=300)
        self.embeds, self.origin, self.entries, self.page = embeds, origin, entries, 0
        if not entries:
            self.remove_item(self.save_btn)
        self.sync()

    def current_embed(self) -> discord.Embed:
        e = self.embeds[self.page]
        e.set_footer(text=f"{self.page + 1}/{len(self.embeds)}")
        return e

    def sync(self):
        self.prev_btn.disabled = self.page == 0
        self.next_btn.disabled = self.page >= len(self.embeds) - 1

    async def interaction_check(self, i: discord.Interaction) -> bool:
        return i.user.id == OWNER_ID

    async def on_timeout(self):
        try:
            await self.origin.edit_original_response(view=None)
        except discord.HTTPException:
            pass

    @discord.ui.button(label="◀", style=discord.ButtonStyle.secondary)
    async def prev_btn(self, i: discord.Interaction, button: discord.ui.Button):
        self.page -= 1
        self.sync()
        await i.response.edit_message(embed=self.current_embed(), view=self)

    @discord.ui.button(label="Save all to list", style=discord.ButtonStyle.success)
    async def save_btn(self, i: discord.Interaction, button: discord.ui.Button):
        await i.response.defer()
        for uid, _, _ in self.entries:
            await ensure_person(uid)
        self.embeds = [await render_entry(*e) for e in self.entries]
        button.disabled = True
        button.label = "Saved"
        await i.edit_original_response(embed=self.current_embed(), view=self)

    @discord.ui.button(label="▶", style=discord.ButtonStyle.secondary)
    async def next_btn(self, i: discord.Interaction, button: discord.ui.Button):
        self.page += 1
        self.sync()
        await i.response.edit_message(embed=self.current_embed(), view=self)


async def lookup_flow(i: discord.Interaction, ids: list[int], save=False, relation: str | None = None, tag: str | None = None):
    await i.response.defer(ephemeral=True)
    t = norm_tag(tag) if tag else None
    entries = []
    for uid in ids:
        if save:
            await ensure_person(uid)
            if relation:
                await set_relation(uid, relation)
            if t:
                await bot.q("INSERT IGNORE INTO people_tags (user_id, tag) VALUES (%s,%s)", uid, t, write=True)
        try:
            p, changes, _ = await take_snapshot(uid)
        except discord.HTTPException:
            p, changes = None, {}
        entries.append((uid, p, changes))
    embeds = [await render_entry(*e) for e in entries]
    if len(embeds) == 1:
        return await i.followup.send(embed=embeds[0])
    view = ResultPager(embeds, i, None if save else entries)
    await i.followup.send(embed=view.current_embed(), view=view)


@bot.tree.context_menu(name="Lookup mentions")
@owner_only()
async def lookup_mentions(i: discord.Interaction, message: discord.Message):
    ids = list(dict.fromkeys(u.id for u in message.mentions))
    if not ids:
        return await i.response.send_message("That message doesn't mention anyone.", ephemeral=True)
    await lookup_flow(i, ids[:25])


@bot.tree.command(description="Look up one or more people (pick users and/or paste IDs or @mentions)")
@app_commands.describe(
    user="A person", user2="Another person", user3="Another person", user4="Another person", user5="Another person",
    ids="Any number of IDs or @mentions, separated by spaces or commas",
    save="Also add them to your list",
    relation="With save: set their relation",
    tag="With save: tag them (new tags are created automatically)",
)
@app_commands.choices(relation=STATUS_CHOICES)
@owner_only()
async def lookup(
    i: discord.Interaction,
    user: discord.User | None = None,
    user2: discord.User | None = None,
    user3: discord.User | None = None,
    user4: discord.User | None = None,
    user5: discord.User | None = None,
    ids: str | None = None,
    save: bool = False,
    relation: app_commands.Choice[str] | None = None,
    tag: str | None = None,
):
    found = [u.id for u in (user, user2, user3, user4, user5) if u] + parse_ids(ids)
    found = list(dict.fromkeys(found))[:25]
    if not found:
        return await i.response.send_message("Give me at least one user, ID or @mention.", ephemeral=True)
    await lookup_flow(i, found, save, relation.value if relation else None, tag)


lookup.autocomplete("tag")(tag_autocomplete)


async def people_ids(relation: str | None, tag: str | None, limit: int = 200) -> list[int]:
    sql, where, args = "SELECT p.user_id FROM people p", [], []
    if tag:
        sql += " JOIN people_tags t ON t.user_id=p.user_id"
        where.append("t.tag=%s")
        args.append(norm_tag(tag))
    if relation:
        where.append("p.relation=%s")
        args.append(relation)
    if where:
        sql += " WHERE " + " AND ".join(where)
    rows = await bot.q(sql + " ORDER BY p.updated_at DESC LIMIT %s", *args, limit)
    return [r[0] for r in rows]


@bot.tree.command(description="Refresh everyone on your list and browse their profiles")
@app_commands.choices(relation=STATUS_CHOICES)
@owner_only()
async def fetchall(i: discord.Interaction, relation: app_commands.Choice[str] | None = None, tag: str | None = None):
    await i.response.defer(ephemeral=True)
    ids = await people_ids(relation.value if relation else None, tag)
    if not ids:
        return await i.followup.send("Nobody on your list matches that.")
    entries, changed, gone = [], [], []
    for uid in ids:
        try:
            p, changes, _ = await take_snapshot(uid)
        except discord.HTTPException:
            p, changes = None, {}
        entries.append((uid, p, changes))
        if p is None:
            gone.append(uid)
        elif changes:
            changed.append((uid, changes))
    summary = discord.Embed(title="Your list", description=f"{len(ids)} people refreshed", color=0x5865F2)
    if changed:
        lines = [f"<@{u}> · {', '.join(ch)}" for u, ch in changed]
        summary.add_field(name=f"Changed ({len(changed)})", value="\n".join(lines)[:1000], inline=False)
    if gone:
        summary.add_field(name=f"Not found ({len(gone)})", value=" ".join(f"`{u}`" for u in gone)[:1000], inline=False)
    if not changed and not gone:
        summary.add_field(name="Changes", value="Nothing changed since the last check.", inline=False)
    embeds = [summary] + [await render_entry(*e) for e in entries]
    view = ResultPager(embeds, i)
    await i.followup.send(embed=view.current_embed(), view=view)


fetchall.autocomplete("tag")(tag_autocomplete)


@bot.tree.command(description="Set someone's relation (friend, unfriended, blocked…)")
@app_commands.choices(status=STATUS_CHOICES)
@owner_only()
async def status(i: discord.Interaction, user: discord.User, status: app_commands.Choice[str]):
    await set_relation(user.id, status.value)
    await i.response.send_message(f"{user.mention} → **{status.name}**", ephemeral=True)


@bot.tree.command(description="Add or replace a note about someone")
@owner_only()
async def note(i: discord.Interaction, user: discord.User, text: str):
    await ensure_person(user.id)
    await bot.q("UPDATE people SET notes=%s WHERE user_id=%s", text, user.id, write=True)
    await i.response.send_message("Saved.", ephemeral=True)


@bot.tree.command(description="Tell the bot where/how you know someone")
@owner_only()
async def met(i: discord.Interaction, user: discord.User, where: str):
    await ensure_person(user.id)
    await bot.q("UPDATE people SET met_context=%s WHERE user_id=%s", where[:60], user.id, write=True)
    await i.response.send_message(f"{user.mention} → met via **{where[:60]}**", ephemeral=True)


@bot.tree.command(description="Show relation + profile change history for someone")
@owner_only()
async def history(i: discord.Interaction, user: discord.User):
    await i.response.defer(ephemeral=True)
    logs = await bot.q("SELECT status, at FROM status_log WHERE user_id=%s ORDER BY id DESC LIMIT 10", user.id)
    snaps = await bot.q("SELECT taken_at FROM snapshots WHERE user_id=%s ORDER BY id DESC LIMIT 10", user.id)
    out = ["**Relation log**"] + [
        f"{STATUSES.get(s, s)} · {discord.utils.format_dt(aware(a), 'f')}" for s, a in logs
    ]
    out += ["", "**Profile snapshots**"] + [discord.utils.format_dt(aware(a[0]), "f") for a in snaps]
    await i.followup.send("\n".join(out)[:1900] if logs or snaps else "Nothing saved yet.")


@bot.tree.command(description="List tracked people, filter by relation and/or tag")
@app_commands.choices(relation=STATUS_CHOICES)
@owner_only()
async def people(
    i: discord.Interaction,
    relation: app_commands.Choice[str] | None = None,
    tag: str | None = None,
):
    sql = (
        "SELECT p.user_id, p.relation, "
        "(SELECT GROUP_CONCAT(tag ORDER BY tag SEPARATOR ', ') FROM people_tags WHERE user_id=p.user_id) "
        "FROM people p"
    )
    where, args = [], []
    if tag:
        sql += " JOIN people_tags t ON t.user_id=p.user_id"
        where.append("t.tag=%s")
        args.append(norm_tag(tag))
    if relation:
        where.append("p.relation=%s")
        args.append(relation.value)
    if where:
        sql += " WHERE " + " AND ".join(where)
    sql += " ORDER BY p.updated_at DESC LIMIT 40"
    rows = await bot.q(sql, *args)
    text = "\n".join(
        f"<@{uid}> `{uid}` · {STATUSES.get(rel, rel)}" + (f" · {tags}" if tags else "")
        for uid, rel, tags in rows
    ) or "Nobody found."
    await i.response.send_message(text[:1900], ephemeral=True)


people.autocomplete("tag")(tag_autocomplete)


@bot.tree.command(description="Upcoming birthdays")
@owner_only()
async def upcoming(i: discord.Interaction, days: app_commands.Range[int, 1, 366] = 60):
    rows = await bot.q(
        "SELECT user_id, birthday_month, birthday_day, birthday_year FROM people WHERE birthday_month IS NOT NULL"
    )
    today = dt.datetime.now(TZ).date()
    found = []
    for uid, mo, d, y in rows:
        nxt = next_birthday(mo, d, today)
        delta = (nxt - today).days
        if delta <= days:
            found.append((delta, uid, nxt, y))
    found.sort()
    lines = []
    for delta, uid, nxt, y in found[:25]:
        when = "**today**" if delta == 0 else f"in {delta}d"
        age = f" · turns {nxt.year - y}" if y else ""
        lines.append(f"{when} · <@{uid}> · {calendar.month_abbr[nxt.month]} {nxt.day}{age}")
    await i.response.send_message(
        "\n".join(lines) or f"No birthdays in the next {days} days.", ephemeral=True
    )

tag_group = app_commands.Group(name="tag", description="Tag the people you track")


@tag_group.command(name="add", description="Tag someone (new tags are created automatically)")
@owner_only()
async def tag_add(i: discord.Interaction, user: discord.User, tag: str):
    t = norm_tag(tag)
    if not t:
        return await i.response.send_message("Tag can't be empty.", ephemeral=True)
    await ensure_person(user.id)
    await bot.q("INSERT IGNORE INTO people_tags (user_id, tag) VALUES (%s,%s)", user.id, t, write=True)
    tags = await get_tags(user.id)
    await i.response.send_message(f"{user.mention} tagged **{t}**. Tags: {', '.join(tags)}", ephemeral=True)


@tag_group.command(name="remove", description="Remove a tag from someone")
@owner_only()
async def tag_remove(i: discord.Interaction, user: discord.User, tag: str):
    await bot.q("DELETE FROM people_tags WHERE user_id=%s AND tag=%s", user.id, norm_tag(tag), write=True)
    tags = await get_tags(user.id)
    await i.response.send_message(f"Removed. Tags now: {', '.join(tags) or 'none'}", ephemeral=True)


@tag_group.command(name="list", description="List all tags, or one person's tags")
@owner_only()
async def tag_list(i: discord.Interaction, user: discord.User | None = None):
    if user:
        tags = await get_tags(user.id)
        return await i.response.send_message(f"{user.mention}: {', '.join(tags) or 'no tags'}", ephemeral=True)
    rows = await bot.q("SELECT tag, COUNT(*) FROM people_tags GROUP BY tag ORDER BY COUNT(*) DESC, tag")
    text = "\n".join(f"**{t}** · {c}" for t, c in rows) or "No tags yet."
    await i.response.send_message(text[:1900], ephemeral=True)


tag_add.autocomplete("tag")(tag_autocomplete)
tag_remove.autocomplete("tag")(tag_autocomplete)
bot.tree.add_command(tag_group)

ARCHIVE_COLS = "message_id, author_id, author_name, author_avatar, content, jump_url, channel_id, guild_id, message_ts, saved_at"


def like_escape(text: str) -> str:
    return re.sub(r"([\\%_])", r"\\\1", text)


async def archive_fetch(where: str = "", args=(), order: str = "ORDER BY saved_at DESC", limit: int = 200) -> list[dict]:
    rows = await bot.q(f"SELECT {ARCHIVE_COLS} FROM archive {where} {order} LIMIT %s", *args, limit)
    docs = [dict(zip(ARCHIVE_COLS.split(", "), r)) for r in rows]
    by_id = {d["message_id"]: [] for d in docs}
    if by_id:
        ph = ",".join(["%s"] * len(by_id))
        frows = await bot.q(
            "SELECT message_id, id, filename, content_type, size, url, compressed, stored_size, (data IS NOT NULL) "
            f"FROM archive_files WHERE message_id IN ({ph}) ORDER BY id",
            *by_id.keys(),
        )
        for mid, fid, fn, ct, size, url, comp, ssize, has in frows:
            by_id[mid].append({
                "filename": fn, "content_type": ct, "size": size, "url": url,
                "compressed": bool(comp), "stored_size": ssize, "file_id": fid if has else None,
            })
    for d in docs:
        d["attachments"] = by_id[d["message_id"]]
    return docs


def archive_embed(d: dict) -> discord.Embed:
    e = discord.Embed(description=d["content"] or "*(no text)*", color=0x5865F2, timestamp=aware(d["message_ts"]))
    e.set_author(name=d["author_name"], icon_url=d.get("author_avatar"))
    if d["attachments"]:
        lines = [
            f"{a['filename']} · {human(a['size'])} · {'stored' if a.get('file_id') else 'link only'}"
            for a in d["attachments"]
        ]
        e.add_field(name="Attachments", value="\n".join(lines)[:1000], inline=False)
    if d.get("jump_url"):
        e.add_field(name="Source", value=f"[jump to message]({d['jump_url']})", inline=False)
    return e


async def load_attachment(entry: dict) -> bytes:
    row = await bot.q("SELECT data, compressed FROM archive_files WHERE id=%s", entry["file_id"], one=True)
    data = bytes(row[0])
    if row[1]:
        data = await asyncio.to_thread(zlib.decompress, data)
    return data


class ArchiveView(discord.ui.View):
    def __init__(self, docs: list[dict], origin: discord.Interaction):
        super().__init__(timeout=300)
        self.docs, self.page, self.origin = docs, 0, origin
        self.sync()

    def current_embed(self) -> discord.Embed:
        d = self.docs[self.page]
        e = archive_embed(d)
        e.set_footer(text=f"{self.page + 1}/{len(self.docs)} · ID {d['message_id']}")
        return e

    def sync(self):
        self.prev_btn.disabled = self.page == 0
        self.next_btn.disabled = self.page >= len(self.docs) - 1
        self.files_btn.disabled = not any(a.get("file_id") for a in self.docs[self.page]["attachments"])

    async def interaction_check(self, i: discord.Interaction) -> bool:
        return i.user.id == OWNER_ID

    async def on_timeout(self):
        try:
            await self.origin.edit_original_response(view=None)
        except discord.HTTPException:
            pass

    @discord.ui.button(label="◀", style=discord.ButtonStyle.secondary)
    async def prev_btn(self, i: discord.Interaction, button: discord.ui.Button):
        self.page -= 1
        self.sync()
        await i.response.edit_message(embed=self.current_embed(), view=self)

    @discord.ui.button(label="Files", emoji="📎", style=discord.ButtonStyle.primary)
    async def files_btn(self, i: discord.Interaction, button: discord.ui.Button):
        await i.response.defer(ephemeral=True)
        for a in self.docs[self.page]["attachments"]:
            if a.get("file_id"):
                data = await load_attachment(a)
                await i.followup.send(file=discord.File(io.BytesIO(data), filename=a["filename"]), ephemeral=True)

    @discord.ui.button(label="▶", style=discord.ButtonStyle.secondary)
    async def next_btn(self, i: discord.Interaction, button: discord.ui.Button):
        self.page += 1
        self.sync()
        await i.response.edit_message(embed=self.current_embed(), view=self)


def author_clause(author: str | None):
    if not author:
        return [], []
    if author.isdigit():
        return ["author_id=%s"], [int(author)]
    return ["author_name LIKE %s"], [f"%{like_escape(author)}%"]


async def author_autocomplete(i: discord.Interaction, current: str):
    if i.user.id != OWNER_ID:
        return []
    cur = current.strip()
    where, args = "", []
    if cur:
        conds, args = ["a.author_name LIKE %s"], [f"%{like_escape(cur)}%"]
        if cur.isdigit():
            conds.append("a.author_id=%s")
            args.append(int(cur))
        where = "WHERE " + " OR ".join(conds)
    rows = await bot.q(
        "SELECT a.author_id, "
        "(SELECT author_name FROM archive WHERE author_id=a.author_id ORDER BY saved_at DESC LIMIT 1), "
        f"COUNT(*) c FROM archive a {where} GROUP BY a.author_id ORDER BY c DESC LIMIT 25",
        *args,
    )
    return [app_commands.Choice(name=f"{name} ({n})"[:100], value=str(aid)) for aid, name, n in rows]


INSERT_FILE = (
    "INSERT INTO archive_files (message_id, filename, content_type, size, url, compressed, stored_size, data) "
    "VALUES (%s,%s,%s,%s,%s,%s,%s,%s)"
)


@bot.tree.context_menu(name="Save to archive")
@owner_only()
async def save_archive(i: discord.Interaction, message: discord.Message):
    await i.response.defer(ephemeral=True)
    if await bot.q("SELECT 1 FROM archive WHERE message_id=%s", message.id, one=True):
        d = (await archive_fetch("WHERE message_id=%s", [message.id], "", 1))[0]
        return await i.followup.send("Already saved.", embed=archive_embed(d))

    created = message.created_at.astimezone(dt.timezone.utc).replace(tzinfo=None)
    await bot.q(
        f"INSERT INTO archive ({ARCHIVE_COLS}) VALUES (%s,%s,%s,%s,%s,%s,%s,%s,%s,%s)",
        message.id, message.author.id, message.author.display_name[:100], message.author.display_avatar.url,
        message.content, message.jump_url, message.channel.id if message.channel else None,
        message.guild.id if message.guild else None, created, now(), write=True,
    )

    atts, raw_total, stored_total = [], 0, 0
    for a in message.attachments:
        entry = {
            "filename": a.filename, "content_type": a.content_type, "size": a.size, "url": a.url,
            "file_id": None, "compressed": False, "stored_size": 0,
        }
        stored = False
        if a.size <= MAX_ATTACH:
            try:
                data = await a.read()
                payload, compressed = data, False
                if not (a.content_type or "").startswith(INCOMPRESSIBLE):
                    comp = await asyncio.to_thread(zlib.compress, data, 9)
                    if len(comp) < len(data) * 0.95:
                        payload, compressed = comp, True
                await bot.q(
                    INSERT_FILE, message.id, a.filename[:255], a.content_type, a.size, a.url,
                    int(compressed), len(payload), payload, write=True,
                )
                entry.update(file_id=True, compressed=compressed, stored_size=len(payload))
                raw_total += len(data)
                stored_total += len(payload)
                stored = True
            except Exception as ex:
                print(f"attachment {a.filename} failed: {ex!r}")
        if not stored:
            await bot.q(
                INSERT_FILE, message.id, a.filename[:255], a.content_type, a.size, a.url, 0, 0, None, write=True
            )
        atts.append(entry)

    doc = {
        "message_id": message.id, "author_name": message.author.display_name,
        "author_avatar": message.author.display_avatar.url, "content": message.content,
        "attachments": atts, "jump_url": message.jump_url, "message_ts": created,
    }
    summary = "Saved."
    if atts:
        n = sum(1 for a in atts if a["file_id"])
        summary += f" {n}/{len(atts)} attachment(s) stored ({human(raw_total)} → {human(stored_total)})."
    await i.followup.send(summary, embed=archive_embed(doc))


archive_group = app_commands.Group(name="archive", description="Your message archive")


@archive_group.command(name="search", description="Search the archive by text and/or author")
@owner_only()
async def archive_search(i: discord.Interaction, text: str | None = None, author: str | None = None):
    await i.response.defer(ephemeral=True)
    where, args = author_clause(author)
    if text:
        where.append("content LIKE %s")
        args.append(f"%{like_escape(text)}%")
    docs = await archive_fetch("WHERE " + " AND ".join(where) if where else "", args)
    if not docs:
        return await i.followup.send("Nothing found.")
    view = ArchiveView(docs, i)
    await i.followup.send(embed=view.current_embed(), view=view)


@archive_group.command(name="random", description="Show a random archived message")
@owner_only()
async def archive_random(i: discord.Interaction, author: str | None = None):
    await i.response.defer(ephemeral=True)
    where, args = author_clause(author)
    docs = await archive_fetch("WHERE " + " AND ".join(where) if where else "", args, "ORDER BY RAND()", 1)
    if not docs:
        return await i.followup.send("Archive is empty.")
    view = ArchiveView(docs, i)
    await i.followup.send(embed=view.current_embed(), view=view)


@archive_group.command(name="delete", description="Delete an archived message by its ID (shown in the footer)")
@owner_only()
async def archive_delete(i: discord.Interaction, message_id: str):
    if not message_id.strip().isdigit():
        return await i.response.send_message("That's not a valid ID.", ephemeral=True)
    n = await bot.q("DELETE FROM archive WHERE message_id=%s", int(message_id), write=True)
    await i.response.send_message("Deleted (including stored attachments)." if n else "Not found.", ephemeral=True)


archive_search.autocomplete("author")(author_autocomplete)
archive_random.autocomplete("author")(author_autocomplete)
bot.tree.add_command(archive_group)


bot.run(TOKEN)
