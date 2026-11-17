// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;

namespace Follower;

public sealed class Row
{
    public long Id { get; init; }
    public string Name { get; init; } = "";
    public string Subtitle { get; init; } = "";
    public ImageSource? Avatar { get; init; }
    public Visibility SaveVisibility { get; init; } = Visibility.Collapsed;
    public object? Data { get; init; }
}

public static class Ui
{
    public static string CdnPng(string url, int size)
    {
        if (!(url.Contains("discordapp.") || url.Contains("discord.com"))) return url;
        var b = url.Split('?')[0];
        if (b.EndsWith(".webp") || b.EndsWith(".gif")) b = b[..b.LastIndexOf('.')] + ".png";
        var want = 32;
        while (want < size * 2 && want < 1024) want <<= 1;
        return $"{b}?size={want}";
    }

    public static BitmapImage? Img(string? url, int size = 96)
    {
        if (string.IsNullOrWhiteSpace(url)) return null;
        try { return new BitmapImage(new Uri(CdnPng(url, size))) { DecodePixelWidth = size }; }
        catch { return null; }
    }

    public static async Task<bool> ConfirmAsync(XamlRoot root, string title, string body, string primary)
    {
        var dlg = new ContentDialog
        {
            XamlRoot = root, Title = title, Content = body,
            PrimaryButtonText = primary, CloseButtonText = "Cancel", DefaultButton = ContentDialogButton.Close,
        };
        return await dlg.ShowAsync() == ContentDialogResult.Primary;
    }

    public static TextBlock Muted(string text) => new()
    {
        Text = text, TextWrapping = TextWrapping.Wrap, IsTextSelectionEnabled = true,
        Style = (Style)Application.Current.Resources["CaptionTextBlockStyle"],
        Foreground = (Brush)Application.Current.Resources["TextFillColorSecondaryBrush"],
    };

    public static Grid Prop(string key, string value)
    {
        var g = new Grid { ColumnSpacing = 16, Margin = new Thickness(0, 4, 0, 4) };
        g.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(150) });
        g.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        g.Children.Add(new TextBlock { Text = key, Foreground = (Brush)Application.Current.Resources["TextFillColorSecondaryBrush"], TextWrapping = TextWrapping.Wrap });
        var v = new TextBlock { Text = value, IsTextSelectionEnabled = true, TextWrapping = TextWrapping.Wrap };
        Grid.SetColumn(v, 1);
        g.Children.Add(v);
        return g;
    }

    public static TextBlock Heading(string text) => new()
    {
        Text = text, Style = (Style)Application.Current.Resources["SubtitleTextBlockStyle"], Margin = new Thickness(0, 0, 0, 8),
    };

    public static void Fit(ScrollViewer sv, FrameworkElement column, double max = 820)
    {
        void Apply(double w) { if (w > 0) column.Width = Math.Min(max, w); }
        sv.SizeChanged += (_, e) => Apply(e.NewSize.Width);
        sv.Loaded += (_, _) => Apply(sv.ActualWidth);
    }

    public static string Trunc(string s, int n) => s.Length > n ? s[..n] + "…" : s;
}
