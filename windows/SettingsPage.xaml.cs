// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using Follower.Data;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace Follower;

public sealed partial class SettingsPage : Page
{
    public SettingsPage()
    {
        InitializeComponent();
        Ui.Fit(Scroll, Column);
        var c = AppState.Config;
        UrlBox.Text = c.Url;
        TokenBox.Password = c.Token;
        TzBox.Text = c.Timezone;
        var v = System.Reflection.Assembly.GetExecutingAssembly().GetName().Version?.ToString(3) ?? "";
        AboutText.Text = $"Follower {v}\n\u00A9 2026 Thom (thom2305)";
    }

    Config Current() => new(UrlBox.Text, TokenBox.Password, TzBox.Text);

    async void SaveTest_Click(object sender, RoutedEventArgs e) => await AppState.SaveConfigAsync(Current(), true);
    async void Save_Click(object sender, RoutedEventArgs e) => await AppState.SaveConfigAsync(Current(), false);

    async void Licenses_Click(object sender, RoutedEventArgs e)
    {
        string text;
        try { text = await File.ReadAllTextAsync(Path.Combine(AppContext.BaseDirectory, "THIRD_PARTY_NOTICES.txt")); }
        catch { text = "License text unavailable."; }
        var dlg = new ContentDialog
        {
            XamlRoot = XamlRoot, Title = "Open-source licenses", CloseButtonText = "Close",
            Content = new ScrollViewer
            {
                MaxHeight = 420,
                Content = new TextBlock { Text = text, TextWrapping = TextWrapping.Wrap, IsTextSelectionEnabled = true },
            },
        };
        await dlg.ShowAsync();
    }
}
