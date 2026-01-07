use std::{error::Error, fs::write, process::Command};

use tempfile::NamedTempFile;

/// Script with marker
fn generate_script(js_code: &str, marker: &str) -> String {
    let print_marker = format!(r#"print("{marker}");"#);

    format!("{print_marker}\n{js_code}\n{print_marker}")
}

/// Search for windows matching the given name pattern
pub fn search_windows_kde(app_name: &str) -> Result<String, Box<dyn Error>> {
    execute_kwin_script(&format!(
        r#"
        const query = new RegExp("{app_name}", "i")
        workspace.windowList().forEach((client) => {{
            if (client.caption.search(query) !== -1) {{
                print(client.internalId);
            }}
        }});
        "#,
    ))
}

/// Activate (focus and raise) the window with the given UUID
pub fn window_activate_kde(window_uuid: &String) -> Result<(), Box<dyn Error>> {
    let confirmation = uuid::Uuid::new_v4().to_string();
    let result = execute_kwin_script(&format!(
        r#"
        workspace.windowList().forEach((client) => {{
            if (`${{client.internalId}}` === "{window_uuid}") {{
                print("{confirmation}");
                workspace.activeWindow = client;
            }}
        }});
        "#
    ))?;

    if result.is_empty() || !result.contains(&confirmation) {
        return Err("Window not found or not activated".into());
    }

    Ok(())
}

fn create_temp_file(data: &str) -> Result<NamedTempFile, Box<dyn Error>> {
    let tempfile = tempfile::Builder::new()
        .prefix("kwinscript-")
        .suffix(".js")
        .tempfile()?;

    write(&tempfile, data)?;

    Ok(tempfile)
}

/// Execute a `KWin` script
fn execute_kwin_script(script_content: &str) -> Result<String, Box<dyn Error>> {
    let marker = uuid::Uuid::new_v4().to_string();
    let tmp_file = create_temp_file(&generate_script(script_content, &marker))?;
    let script_path = tmp_file.path();

    macro_rules! new_cmd {
        () => {
            std::process::Command::new("qdbus").arg("org.kde.KWin")
        };
    }

    // Load script
    let output = new_cmd!()
        .arg("/Scripting")
        .arg("org.kde.kwin.Scripting.loadScript")
        .arg(script_path)
        .output()?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }

    let object_path = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<i32>()
        .ok()
        .and_then(|id| {
            if id >= 0 {
                Some(format!("/Scripting/Script{id}"))
            } else {
                None
            }
        })
        .ok_or(std::io::Error::other("Can't load KDE Script"))?;

    // Run the script
    match new_cmd!()
        .arg(&object_path)
        .arg("org.kde.kwin.Script.run")
        .output()
    {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            return Err(
                format!("Script failed: {}", String::from_utf8_lossy(&output.stderr)).into(),
            );
        }
        Err(e) => return Err(format!("Failed to start script: {e}").into()),
    }

    // Extract data from journal
    let journal_output = Command::new("journalctl")
        .arg("--user")
        .arg("-u")
        .arg("plasma-kwin_wayland.service")
        .arg("-n")
        .arg("10")
        .arg("--no-pager")
        .arg("-o")
        .arg("cat")
        .output()?;

    // Stop and unload the script
    new_cmd!()
        .arg(&object_path)
        .arg("org.kde.kwin.Script.stop")
        .output()?;
    new_cmd!()
        .arg("/Scripting")
        .arg("org.kde.kwin.Scripting.unloadScript")
        .arg(script_path)
        .output()?;

    // Extract the printed output from journal
    let jrnl = String::from_utf8_lossy(&journal_output.stdout);
    Ok(jrnl
        .lines()
        .rev()
        .skip_while(|line| !line.contains(&marker))
        .skip(1)
        .take_while(|line| !line.contains(&marker))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .into())
}
