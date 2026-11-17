// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace Follower.Data;

public sealed record Config(string Url = "", string Token = "", string Timezone = "America/Toronto")
{
    public bool Ready => Url.Trim().Length > 0 && Token.Trim().Length > 0;
    public TimeZoneInfo Zone => Util.Zone(Timezone);
}

public sealed class ConfigStore
{
    readonly string path = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "Follower", "config.json");

    public Config Load()
    {
        try
        {
            if (!File.Exists(path)) return new Config();
            var o = JsonNode.Parse(File.ReadAllText(path)) as JsonObject;
            var token = "";
            if (o.Str("token_dpapi") is { } enc)
            {
                try { token = Encoding.UTF8.GetString(ProtectedData.Unprotect(Convert.FromBase64String(enc), null, DataProtectionScope.CurrentUser)); }
                catch { /* different user/machine: ask for the token again */ }
            }
            return new Config(o.Str("url") ?? "", token, o.Str("tz") ?? "America/Toronto");
        }
        catch { return new Config(); }
    }

    public void Save(Config c)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        var prot = ProtectedData.Protect(Encoding.UTF8.GetBytes(c.Token.Trim()), null, DataProtectionScope.CurrentUser);
        var o = new JsonObject
        {
            ["url"] = c.Url.Trim(),
            ["token_dpapi"] = Convert.ToBase64String(prot),
            ["tz"] = c.Timezone.Trim(),
        };
        File.WriteAllText(path, o.ToJsonString(new JsonSerializerOptions { WriteIndented = true }));
    }
}
