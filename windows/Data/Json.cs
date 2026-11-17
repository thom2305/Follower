// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using System.Globalization;
using System.Text.Json.Nodes;

namespace Follower.Data;

public static class J
{
    public static JsonNode? Get(this JsonNode? n, string k) =>
        n is JsonObject o && o.TryGetPropertyValue(k, out var v) ? v : null;

    public static string? Str(this JsonNode? n, string k) =>
        n.Get(k) is JsonValue v && v.TryGetValue<string>(out var s) ? s : null;

    public static long? Int(this JsonNode? n, string k) =>
        n.Get(k) is JsonValue v && v.TryGetValue<long>(out var x) ? x : null;

    public static bool Bool(this JsonNode? n, string k) =>
        n.Get(k) is JsonValue v && v.TryGetValue<bool>(out var b) && b;

    public static JsonObject? Obj(this JsonNode? n, string k) => n.Get(k) as JsonObject;

    public static IEnumerable<JsonObject> Objs(this JsonNode? n, string k) =>
        n.Get(k) is JsonArray a ? a.OfType<JsonObject>() : Enumerable.Empty<JsonObject>();

    public static List<string> Strings(this JsonNode? n, string k)
    {
        var list = new List<string>();
        if (n.Get(k) is JsonArray a)
            foreach (var x in a)
                if (x is JsonValue v && v.TryGetValue<string>(out var s)) list.Add(s);
        return list;
    }

    public static long? Id64(this JsonNode? n, string k)
    {
        if (n.Get(k) is not JsonValue v) return null;
        if (v.TryGetValue<string>(out var s)) return long.TryParse(s, out var l) ? l : null;
        return v.TryGetValue<long>(out var x) ? x : null;
    }

    public static DateTimeOffset? Date(this JsonNode? n, string k) => n.Str(k) is { } s ? Util.ParseIso(s) : null;
}
