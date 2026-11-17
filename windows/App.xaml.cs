// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using Microsoft.UI.Xaml;

namespace Follower;

public partial class App : Application
{
    public static MainWindow MainWin { get; private set; } = null!;

    public App()
    {
        InitializeComponent();
        UnhandledException += (_, e) =>
        {
            e.Handled = true;
            AppState.Say("Unexpected error: " + e.Message);
        };
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        MainWin = new MainWindow();
        MainWin.Activate();
    }
}
