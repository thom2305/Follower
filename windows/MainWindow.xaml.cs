// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using Microsoft.UI;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml.Media.Imaging;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Navigation;

namespace Follower;

public sealed partial class MainWindow : Window
{
    static readonly Dictionary<string, Type> Pages = new()
    {
        ["people"] = typeof(PeoplePage),
        ["lookup"] = typeof(LookupPage),
        ["birthdays"] = typeof(BirthdaysPage),
        ["archive"] = typeof(ArchivePage),
    };

    readonly Microsoft.UI.Dispatching.DispatcherQueueTimer toastTimer;
    bool syncing;

    public MainWindow()
    {
        InitializeComponent();
        Title = "Follower";
        ExtendsContentIntoTitleBar = true;
        SetTitleBar(AppTitleBar);
        try
        {
            AppWindow.TitleBar.ButtonBackgroundColor = Colors.Transparent;
            AppWindow.TitleBar.ButtonInactiveBackgroundColor = Colors.Transparent;
            AppIcon.Source = new BitmapImage(new Uri(Path.Combine(AppContext.BaseDirectory, "Assets", "app.png")));
        }
        catch { }
        try { SystemBackdrop = new MicaBackdrop(); } catch { /* Windows 10: plain background */ }
        try
        {
            AppWindow.SetIcon(Path.Combine(AppContext.BaseDirectory, "Assets", "app.ico"));
            AppWindow.Resize(new Windows.Graphics.SizeInt32(1100, 780));
        }
        catch { }

        toastTimer = DispatcherQueue.CreateTimer();
        toastTimer.Interval = TimeSpan.FromSeconds(4.5);
        toastTimer.IsRepeating = false;
        toastTimer.Tick += (_, _) => Toast.IsOpen = false;
        AppState.Toast = msg =>
        {
            DispatcherQueue.TryEnqueue(() =>
            {
                Toast.Message = msg;
                Toast.IsOpen = true;
                toastTimer.Stop();
                toastTimer.Start();
            });
        };

        if (AppState.Config.Ready)
        {
            Nav.SelectedItem = Nav.MenuItems[0];
            _ = AppState.LoadPeopleAsync();
        }
        else
        {
            Nav.SelectedItem = Nav.SettingsItem;
        }
    }

    void Nav_SelectionChanged(NavigationView sender, NavigationViewSelectionChangedEventArgs args)
    {
        if (syncing) return;
        Type? target = args.IsSettingsSelected
            ? typeof(SettingsPage)
            : (args.SelectedItemContainer?.Tag is string t && Pages.TryGetValue(t, out var p) ? p : null);
        if (target is null) return;
        ContentFrame.BackStack.Clear();
        ContentFrame.Navigate(target);
    }

    void Nav_BackRequested(NavigationView sender, NavigationViewBackRequestedEventArgs args)
    {
        if (ContentFrame.CanGoBack) ContentFrame.GoBack();
    }

    void ContentFrame_Navigated(object sender, NavigationEventArgs e)
    {
        Nav.IsBackEnabled = ContentFrame.CanGoBack;
        syncing = true;
        try
        {
            if (e.SourcePageType == typeof(SettingsPage)) Nav.SelectedItem = Nav.SettingsItem;
            else
            {
                var key = Pages.FirstOrDefault(kv => kv.Value == e.SourcePageType).Key;
                if (key is not null)
                    Nav.SelectedItem = Nav.MenuItems.OfType<NavigationViewItem>().First(i => (string)i.Tag == key);
            }
        }
        finally { syncing = false; }
    }
}
