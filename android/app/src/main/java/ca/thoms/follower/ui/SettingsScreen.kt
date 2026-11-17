// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

@file:OptIn(ExperimentalMaterial3Api::class)

package ca.thoms.follower.ui

import android.content.Intent
import com.google.android.gms.oss.licenses.OssLicensesMenuActivity
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import ca.thoms.follower.AppViewModel
import ca.thoms.follower.data.Config

private const val REPO_URL = "https://github.com/thom2305/Follower"

@Composable
fun SettingsScreen(vm: AppViewModel) {
    val c = vm.config
    var url by remember { mutableStateOf(c.url) }
    var token by remember { mutableStateOf(c.token) }
    var tz by remember { mutableStateOf(c.timezone) }

    fun build() = Config(url, token, tz)

    val context = LocalContext.current
    val uriHandler = LocalUriHandler.current
    var showLicenses by remember { mutableStateOf(false) }
    val version = remember {
        runCatching { context.packageManager.getPackageInfo(context.packageName, 0).versionName }.getOrNull() ?: ""
    }
    val licenseText = remember {
        runCatching { context.assets.open("THIRD_PARTY_NOTICES.txt").bufferedReader().use { it.readText() } }
            .getOrDefault("License text unavailable.")
    }

    ScreenScaffold(title = "Settings") { pad ->
        Column(
            Modifier.padding(pad).verticalScroll(rememberScrollState()).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text("Bot server", style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary)
            OutlinedTextField(
                url, { url = it }, label = { Text("Server URL") }, singleLine = true,
                placeholder = { Text("https://follower.example.com or http://100.x.y.z:8080") },
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                token, { token = it }, label = { Text("API token") }, singleLine = true,
                visualTransformation = PasswordVisualTransformation(), modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(tz, { tz = it }, label = { Text("Timezone") }, singleLine = true, modifier = Modifier.fillMaxWidth())

            Row(Modifier.padding(top = 8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = { vm.saveConfig(build(), test = true) }) { Text("Save & test") }
                OutlinedButton(onClick = { vm.saveConfig(build(), test = false) }) { Text("Save") }
            }
            Text(
                "The token is stored only on this device (app-private storage, excluded from backups). " +
                    "Prefer an https:// URL, or a private network such as Tailscale, since http:// sends the token unencrypted.",
                style = MaterialTheme.typography.bodySmall,
            )

            HorizontalDivider(Modifier.padding(vertical = 8.dp))
            Text("About", style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary)
            Text("Follower $version", style = MaterialTheme.typography.bodyMedium)
            Text(
                "\u00A9 2026 Thom (thom2305) \u00B7 Licensed under the Apache License 2.0",
                style = MaterialTheme.typography.bodySmall,
            )
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = { uriHandler.openUri(REPO_URL) }) { Text("Source code") }
                OutlinedButton(onClick = {
                    OssLicensesMenuActivity.setActivityTitle("Open-source licenses")
                    context.startActivity(Intent(context, OssLicensesMenuActivity::class.java))
                }) { Text("Open-source licenses") }
            }
        }
    }

    if (showLicenses) {
        AlertDialog(
            onDismissRequest = { showLicenses = false },
            title = { Text("Open-source licenses") },
            text = { Column(Modifier.verticalScroll(rememberScrollState())) { Text(licenseText, style = MaterialTheme.typography.bodySmall) } },
            confirmButton = { TextButton(onClick = { showLicenses = false }) { Text("Close") } },
        )
    }
}
