# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright 2026 Thom (thom2305)
#
# This program is free software: you can redistribute it and/or modify it under
# the terms of the GNU Affero General Public License as published by the Free
# Software Foundation, either version 3 of the License, or (at your option) any
# later version. If you run a modified version of this program for other people
# over a network, you must offer them its source code (AGPL section 13).
# See the LICENSE file in this folder.

import asyncio
import hmac
import json
import os
import zlib

import discord
from aiohttp import web

PEOPLE_COLS = (
    "user_id, relation, notes, met_context, birthday_month, birthday_day, birthday_year, first_seen, updated_at"
)
ARCHIVE_COLS = (
    "message_id, author_id, author_name, author_avatar, content, jump_url, channel_id, guild_id, message_ts, saved_at"
)


class ApiError(Exception):
    def __init__(self, message: str, status: int = 400):
        super().__init__(message)
        self.status = status


def iso(d):
    return None if d is None else d.replace(tzinfo=None).isoformat(timespec="seconds") + "Z"


def sid(v):
    return None if v is None else str(v)


def fmt_value(v) -> str:
    if v is None:
        return "None"
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (list, tuple)):
        return ", ".join(str(x) for x in v)
    return str(v)


def fmt_changes(changes: dict) -> dict:
    return {k: [fmt_value(a), fmt_value(b)] for k, (a, b) in changes.items()}


def make_app(bot, m, token: str) -> web.Application:
    expected = f"Bearer {token}".encode()

    @web.middleware
    async def guard(request: web.Request, handler):
        supplied = request.headers.get("Authorization", "").encode()
        if not hmac.compare_digest(supplied, expected):
            return web.json_response({"error": "unauthorized"}, status=401)
        try:
            return await handler(request)
        except ApiError as e:
            return web.json_response({"error": str(e)}, status=e.status)
        except web.HTTPException:
            raise
        except discord.HTTPException as e:
            return web.json_response({"error": f"Discord error: {e}"}, status=502)
        except Exception as e:
            print(f"[api] {request.method} {request.path} failed: {e!r}")
            return web.json_response({"error": f"{type(e).__name__}: {e}"}, status=500)

    def uid_of(request: web.Request) -> int:
        try:
            return int(request.match_info["uid"])
        except (KeyError, ValueError):
            raise ApiError("bad user id")

    async def body_of(request: web.Request) -> dict:
        if not request.can_read_body:
            return {}
        try:
            data = await request.json()
        except json.JSONDecodeError:
            raise ApiError("body must be JSON")
        if not isinstance(data, dict):
            raise ApiError("body must be a JSON object")
        return data

    def person_row_to_json(row, tags) -> dict:
        uid, relation, notes, met, bm, bd, by, first_seen, updated_at = row
        return {
            "user_id": str(uid), "relation": relation, "notes": notes, "met_context": met,
            "birthday_month": bm, "birthday_day": bd, "birthday_year": by,
            "first_seen": iso(first_seen), "updated_at": iso(updated_at), "tags": tags,
        }

    async def person_json(uid: int):
        row = await bot.q(f"SELECT {PEOPLE_COLS} FROM people WHERE user_id=%s", uid, one=True)
        if not row:
            return None
        return person_row_to_json(row, await m.get_tags(uid))

    async def snap(uid: int):
        try:
            return await m.take_snapshot(uid)
        except discord.HTTPException:
            return None, {}, False

    def snap_json(p, changes, first) -> dict:
        return {"found": p is not None, "profile": p, "changes": fmt_changes(changes), "is_first": first}

    async def ping(request):
        p = await bot.q("SELECT COUNT(*) FROM people", one=True)
        a = await bot.q("SELECT COUNT(*) FROM archive", one=True)
        return web.json_response({"ok": True, "people": p[0], "archive": a[0]})

    async def tags_all(request):
        rows = await bot.q("SELECT tag, COUNT(*) FROM people_tags GROUP BY tag ORDER BY COUNT(*) DESC, tag")
        return web.json_response({"tags": [{"tag": t, "count": c} for t, c in rows]})

    async def people_list(request):
        rows = await bot.q(f"SELECT {PEOPLE_COLS} FROM people ORDER BY updated_at DESC")
        tag_rows = await bot.q("SELECT user_id, tag FROM people_tags ORDER BY tag")
        snaps = await bot.q(
            "SELECT s.user_id, s.data FROM snapshots s "
            "JOIN (SELECT MAX(id) mid FROM snapshots GROUP BY user_id) mx ON s.id = mx.mid"
        )
        tags: dict[int, list[str]] = {}
        for u, t in tag_rows:
            tags.setdefault(u, []).append(t)
        profiles = {u: json.loads(d) for u, d in snaps}
        out = []
        for row in rows:
            item = person_row_to_json(row, tags.get(row[0], []))
            item["profile"] = profiles.get(row[0])
            out.append(item)
        return web.json_response({"people": out})

    async def person_get(request):
        uid = uid_of(request)
        return web.json_response({"person": await person_json(uid), "profile": await m.last_profile(uid)})

    async def person_refresh(request):
        uid = uid_of(request)
        return web.json_response(snap_json(*await snap(uid)))

    async def person_update(request):
        uid = uid_of(request)
        body = await body_of(request)
        sets, args = [], []
        if "notes" in body:
            sets.append("notes=%s")
            args.append(str(body["notes"] or "").strip()[:1000] or None)
        if "met_context" in body:
            sets.append("met_context=%s")
            args.append(str(body["met_context"] or "").strip()[:60] or None)
        if "birthday" in body:
            try:
                y, mo, d = m.parse_birthday(str(body["birthday"] or ""))
            except ValueError:
                raise ApiError("Birthday must look like 1999-03-05 or 03-05")
            sets += ["birthday_month=%s", "birthday_day=%s", "birthday_year=%s"]
            args += [mo, d, y]
        rel = body.get("relation")
        if rel is not None and rel not in m.STATUSES:
            raise ApiError(f"unknown relation '{rel}'")
        await m.ensure_person(uid)
        if rel is not None:
            await m.set_relation(uid, rel)
        if sets:
            await bot.q(f"UPDATE people SET {', '.join(sets)} WHERE user_id=%s", *args, uid, write=True)
        return web.json_response({"person": await person_json(uid)})

    async def person_delete(request):
        uid = uid_of(request)
        await bot.q("DELETE FROM people WHERE user_id=%s", uid, write=True)
        await bot.q("DELETE FROM people_tags WHERE user_id=%s", uid, write=True)
        return web.json_response({"ok": True})

    async def people_save(request):
        body = await body_of(request)
        ids = [int(x) for x in body.get("ids", [])][:50]
        for uid in ids:
            await m.ensure_person(uid)
        return web.json_response({"saved": len(ids)})

    async def tag_add(request):
        uid = uid_of(request)
        t = m.norm_tag(str((await body_of(request)).get("tag", "")))
        if not t:
            raise ApiError("tag can't be empty")
        await m.ensure_person(uid)
        await bot.q("INSERT IGNORE INTO people_tags (user_id, tag) VALUES (%s,%s)", uid, t, write=True)
        return web.json_response({"tags": await m.get_tags(uid)})

    async def tag_remove(request):
        uid = uid_of(request)
        await bot.q(
            "DELETE FROM people_tags WHERE user_id=%s AND tag=%s", uid, m.norm_tag(request.match_info["tag"]), write=True
        )
        return web.json_response({"tags": await m.get_tags(uid)})

    async def person_history(request):
        uid = uid_of(request)
        logs = await bot.q("SELECT status, at FROM status_log WHERE user_id=%s ORDER BY id DESC LIMIT 20", uid)
        snaps = await bot.q("SELECT taken_at, data FROM snapshots WHERE user_id=%s ORDER BY id ASC", uid)
        prev, out = None, []
        for at, data in snaps:
            cur = json.loads(data)
            out.append({
                "at": iso(at), "is_first": prev is None,
                "changes": fmt_changes(m.diff(prev, cur)) if prev else {},
            })
            prev = cur
        return web.json_response({
            "statuses": [{"status": s, "at": iso(a)} for s, a in logs],
            "profile_changes": out[::-1][:20],
        })

    async def lookup(request):
        body = await body_of(request)
        ids = list(dict.fromkeys(int(x) for x in body.get("ids", [])))[:25]
        if not ids:
            raise ApiError("no ids given")
        save = bool(body.get("save"))
        relation = body.get("relation")
        if relation is not None and relation not in m.STATUSES:
            raise ApiError(f"unknown relation '{relation}'")
        tag = m.norm_tag(str(body["tag"])) if body.get("tag") else None
        results = []
        for uid in ids:
            if save:
                await m.ensure_person(uid)
                if relation:
                    await m.set_relation(uid, relation)
                if tag:
                    await bot.q("INSERT IGNORE INTO people_tags (user_id, tag) VALUES (%s,%s)", uid, tag, write=True)
            p, changes, first = await snap(uid)
            entry = snap_json(p, changes, first)
            entry["user_id"] = str(uid)
            entry["person"] = await person_json(uid)
            results.append(entry)
        return web.json_response({"results": results})

    def author_clause(author):
        a = (author or "").strip()
        if not a:
            return [], []
        if a.isdigit():
            return ["author_id=%s"], [int(a)]
        return ["author_name LIKE %s"], [f"%{m.like_escape(a)}%"]

    async def fetch_archive(where="", args=(), order="ORDER BY saved_at DESC", limit=200):
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
                    "id": fid, "filename": fn, "content_type": ct, "size": int(size), "url": url,
                    "compressed": bool(comp), "stored_size": int(ssize), "stored": bool(has),
                })
        out = []
        for d in docs:
            out.append({
                "message_id": str(d["message_id"]), "author_id": str(d["author_id"]),
                "author_name": d["author_name"], "author_avatar": d["author_avatar"], "content": d["content"],
                "jump_url": d["jump_url"], "channel_id": sid(d["channel_id"]), "guild_id": sid(d["guild_id"]),
                "message_ts": iso(d["message_ts"]), "saved_at": iso(d["saved_at"]),
                "attachments": by_id[d["message_id"]],
            })
        return out

    async def archive_search(request):
        where, args = author_clause(request.query.get("author"))
        text = (request.query.get("text") or "").strip()
        if text:
            where.append("content LIKE %s")
            args.append(f"%{m.like_escape(text)}%")
        limit = max(1, min(int(request.query.get("limit", 200)), 500))
        msgs = await fetch_archive("WHERE " + " AND ".join(where) if where else "", args, limit=limit)
        return web.json_response({"messages": msgs})

    async def archive_random(request):
        where, args = author_clause(request.query.get("author"))
        msgs = await fetch_archive("WHERE " + " AND ".join(where) if where else "", args, "ORDER BY RAND()", 1)
        return web.json_response({"message": msgs[0] if msgs else None})

    async def archive_authors(request):
        rows = await bot.q(
            "SELECT a.author_id, "
            "(SELECT author_name FROM archive WHERE author_id=a.author_id ORDER BY saved_at DESC LIMIT 1), "
            "COUNT(*) c FROM archive a GROUP BY a.author_id ORDER BY c DESC LIMIT 50"
        )
        return web.json_response({"authors": [{"id": str(a), "name": n, "count": c} for a, n, c in rows]})

    async def archive_delete(request):
        try:
            mid = int(request.match_info["mid"])
        except ValueError:
            raise ApiError("bad message id")
        n = await bot.q("DELETE FROM archive WHERE message_id=%s", mid, write=True)
        return web.json_response({"deleted": n > 0})

    async def archive_file(request):
        try:
            fid = int(request.match_info["fid"])
        except ValueError:
            raise ApiError("bad file id")
        row = await bot.q(
            "SELECT filename, content_type, data, compressed FROM archive_files WHERE id=%s", fid, one=True
        )
        if not row or row[2] is None:
            raise ApiError("file not stored", 404)
        data = bytes(row[2])
        if row[3]:
            data = await asyncio.to_thread(zlib.decompress, data)
        return web.Response(body=data, headers={"Content-Type": row[1] or "application/octet-stream"})

    app = web.Application(middlewares=[guard], client_max_size=1024 * 1024)
    app.add_routes([
        web.get("/api/ping", ping),
        web.get("/api/tags", tags_all),
        web.get("/api/people", people_list),
        web.post("/api/people/save", people_save),
        web.get("/api/people/{uid}", person_get),
        web.post("/api/people/{uid}", person_update),
        web.delete("/api/people/{uid}", person_delete),
        web.post("/api/people/{uid}/refresh", person_refresh),
        web.get("/api/people/{uid}/history", person_history),
        web.post("/api/people/{uid}/tags", tag_add),
        web.delete("/api/people/{uid}/tags/{tag}", tag_remove),
        web.post("/api/lookup", lookup),
        web.get("/api/archive", archive_search),
        web.get("/api/archive/random", archive_random),
        web.get("/api/archive/authors", archive_authors),
        web.get("/api/archive/files/{fid}", archive_file),
        web.delete("/api/archive/{mid}", archive_delete),
    ])
    return app


async def start(bot, m):
    token = os.environ.get("API_TOKEN", "").strip()
    if not token:
        print("[api] API_TOKEN not set - HTTP API disabled")
        return
    if len(token) < 24:
        print("[api] WARNING: API_TOKEN is short; use something like `python -c \"import secrets;print(secrets.token_urlsafe(32))\"`")
    runner = web.AppRunner(make_app(bot, m, token))
    await runner.setup()
    host, port = os.environ.get("API_HOST", "0.0.0.0"), int(os.environ.get("API_PORT", "8080"))
    await web.TCPSite(runner, host, port).start()
    bot.api_runner = runner
    print(f"[api] listening on {host}:{port}")
