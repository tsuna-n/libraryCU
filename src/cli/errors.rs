use anyhow::Error;

/// Errors follow the configured UI language; automatic mode uses the process locale.
pub fn language() -> String {
    let configured = crate::config::load()
        .map(|loaded| loaded.config.output.language)
        .unwrap_or_else(|_| "en".into());
    if configured != "auto" {
        return configured;
    }
    let locale = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|key| std::env::var(key).ok())
        .find(|value| !value.is_empty())
        .unwrap_or_default();
    if locale.to_ascii_lowercase().starts_with("th") {
        "th".into()
    } else {
        "en".into()
    }
}

/// Render human guidance without serializing parser source snippets or credentials.
pub fn format_error(error: &Error, language: &str) -> String {
    let message = safe_message(error);
    let lower = message.to_ascii_lowercase();
    let (fix, fix_th, next) = if lower.contains("configuration")
        || lower.starts_with("ai.")
        || lower.starts_with("output.language")
        || lower.starts_with("memory.mode")
        || lower.starts_with("scanner.")
        || lower.starts_with("unsupported setting")
    {
        (
            "Correct the named setting in your config file (LBC_CONFIG or the default config path).",
            "แก้การตั้งค่าที่ระบุในไฟล์ config (LBC_CONFIG หรือไฟล์เริ่มต้น)",
            "lbc config show; lbc doctor",
        )
    } else if lower.contains("api_key") && lower.contains("not set") {
        (
            "Set the named API key in your environment, or keep using local commands without --ai.",
            "ตั้งค่า API key ที่ระบุใน environment หรือใช้คำสั่งภายในเครื่องโดยไม่ใส่ --ai",
            "lbc doctor; lbc ask --help",
        )
    } else if lower.contains("provider")
        || lower.contains("openrouter")
        || lower.contains("endpoint")
        || lower.contains("timed out")
        || lower.contains("timeout")
    {
        (
            "Check the provider URL, credentials, network and model; retry or omit --ai for local results.",
            "ตรวจ URL, ข้อมูลรับรอง, เครือข่าย และโมเดลของผู้ให้บริการ แล้วลองใหม่หรือไม่ใส่ --ai",
            "lbc doctor --connectivity; lbc config show",
        )
    } else if lower.contains("error log") || lower.contains("error input") {
        (
            "Pass a readable, nonempty error log file or pipe error output through stdin.",
            "ระบุไฟล์ error log ที่อ่านได้และไม่ว่าง หรือส่งข้อความ error ผ่าน stdin",
            "lbc explain --help; lbc explain --stdin < error.log",
        )
    } else if lower.contains("project path") {
        (
            "Choose an existing project directory that you can read.",
            "เลือกโฟลเดอร์โปรเจกต์ที่มีอยู่และมีสิทธิ์อ่าน",
            "lbc scan --path .; lbc scan --help",
        )
    } else if lower.contains("knowledge validation")
        || lower.contains("invalid yaml")
        || lower.contains("frontmatter")
    {
        (
            "Correct the entry's YAML frontmatter metadata, then rebuild the knowledge index.",
            "แก้ข้อมูล YAML frontmatter ของรายการ แล้วสร้างดัชนีความรู้ใหม่",
            "lbc index --json; lbc doctor",
        )
    } else if lower.contains("knowledge entry") || lower.contains("read-only") {
        (
            "List entries and use the complete source ID; use edit --override for a read-only entry.",
            "ดูรายการและใช้ source ID แบบเต็ม; ใช้ edit --override สำหรับรายการที่อ่านได้อย่างเดียว",
            "lbc list; lbc edit --help",
        )
    } else if lower.contains("package") || lower.contains("manifest") || lower.contains("toml") {
        (
            "Check package.toml and its Markdown documents, then retry with the package directory.",
            "ตรวจ package.toml และเอกสาร Markdown แล้วลองใหม่ด้วยโฟลเดอร์แพ็กเกจ",
            "lbc knowledge install --help; lbc knowledge list",
        )
    } else if lower.contains("question") || lower.contains("query") {
        (
            "Provide a nonempty question or search query within the stated size limit.",
            "ระบุคำถามหรือคำค้นที่ไม่ว่างและมีขนาดไม่เกินที่กำหนด",
            "lbc ask --help; lbc search --help",
        )
    } else if lower.contains("permission") || lower.contains("symlink") {
        (
            "Check file ownership and permissions; use a regular file or directory you control.",
            "ตรวจเจ้าของไฟล์และสิทธิ์การเข้าถึง แล้วใช้ไฟล์หรือโฟลเดอร์ปกติที่คุณควบคุมได้",
            "lbc doctor",
        )
    } else if lower.contains("doctor found") {
        (
            "Resolve the failing checks shown above, then run the doctor again.",
            "แก้รายการตรวจสอบที่ล้มเหลวด้านบน แล้วตรวจอีกครั้ง",
            "lbc doctor; lbc setup --help",
        )
    } else {
        (
            "Review the cause and check the command's required inputs and file permissions before retrying.",
            "ตรวจสาเหตุ ข้อมูลที่คำสั่งต้องใช้ และสิทธิ์เข้าถึงไฟล์ก่อนลองใหม่",
            "lbc --help; lbc doctor",
        )
    };
    if language == "th" {
        format!("สาเหตุ: {message}\nวิธีแก้: {fix_th}\nคำสั่งถัดไป: {next}")
    } else {
        format!("Cause: {message}\nFix: {fix}\nNext: {next}")
    }
}

pub(crate) fn safe_message(error: &Error) -> String {
    // TOML diagnostics include the original line and arbitrary invalid values.
    let mut message = if error.downcast_ref::<toml::de::Error>().is_some() {
        let outer = error.to_string();
        if outer.starts_with("invalid configuration") || outer.starts_with("invalid package.toml") {
            format!("{outer}: invalid TOML syntax or setting type")
        } else {
            "invalid TOML syntax or setting type".into()
        }
    } else {
        format!("{error:#}")
    };
    if message.starts_with("ai.provider must be") {
        message.truncate(message.find(" (found ").unwrap_or(message.len()));
    } else if message.starts_with("unsupported AI provider") {
        message = "unsupported AI provider; choose a supported ai.provider".into();
    } else if message.starts_with("unsupported setting") {
        message = "unsupported setting; use output.language, scanner.max_file_size_kb, scanner.ignore_hidden, memory.mode, ai.provider, ai.model, or ai.base_url".into();
    }
    for name in [
        "OPENAI_API_KEY",
        "OPENROUTER_API_KEY",
        "ZAI_API_KEY",
        "GLM_API_KEY",
        "OLLAMA_API_KEY",
    ] {
        if let Ok(value) = std::env::var(name)
            && !value.trim().is_empty()
        {
            message = message.replace(&value, "[REDACTED]");
        }
    }
    crate::security::redact_sensitive(&message)
}

/// clap's diagnostics can echo arbitrary positional values and invalid credentials.
pub fn format_usage_error(error: &clap::Error, language: &str) -> String {
    use clap::error::{ContextKind, ErrorKind};
    let thai = language == "th";
    let (cause, cause_th) = match error.kind() {
        ErrorKind::MissingRequiredArgument | ErrorKind::MissingSubcommand => (
            "a required argument or command is missing",
            "ขาดอาร์กิวเมนต์หรือคำสั่งที่จำเป็น",
        ),
        ErrorKind::ArgumentConflict => (
            "the supplied options cannot be used together",
            "ตัวเลือกที่ระบุใช้ร่วมกันไม่ได้",
        ),
        ErrorKind::InvalidValue | ErrorKind::ValueValidation => {
            ("an argument has an invalid value", "ค่าอาร์กิวเมนต์ไม่ถูกต้อง")
        }
        ErrorKind::UnknownArgument | ErrorKind::InvalidSubcommand => (
            "an argument or command is not recognized",
            "ไม่รู้จักอาร์กิวเมนต์หรือคำสั่งที่ระบุ",
        ),
        _ => (
            "the command arguments are invalid",
            "อาร์กิวเมนต์ของคำสั่งไม่ถูกต้อง",
        ),
    };
    let mut rendered = if thai {
        format!(
            "lbc: สาเหตุ: {cause_th}\nวิธีแก้: ตรวจชื่อคำสั่ง ตัวเลือก และค่าที่รองรับ\nคำสั่งถัดไป: lbc --help; lbc <command> --help"
        )
    } else {
        format!(
            "lbc: Cause: {cause}\nFix: Check the command, options and allowed values.\nNext: lbc --help; lbc <command> --help"
        )
    };
    // These contexts contain declared names, never rejected values or unknown arguments.
    if matches!(
        error.kind(),
        ErrorKind::MissingRequiredArgument
            | ErrorKind::ArgumentConflict
            | ErrorKind::InvalidValue
            | ErrorKind::ValueValidation
            | ErrorKind::TooManyValues
            | ErrorKind::TooFewValues
            | ErrorKind::WrongNumberOfValues
            | ErrorKind::NoEquals
    ) {
        for (kind, label, label_th) in [
            (ContextKind::InvalidArg, "Argument", "อาร์กิวเมนต์"),
            (ContextKind::PriorArg, "Conflicts with", "ขัดแย้งกับ"),
            (ContextKind::ValidValue, "Allowed values", "ค่าที่รองรับ"),
        ] {
            if let Some(value) = error.get(kind) {
                rendered.push_str(&format!(
                    "\n{}: {value}",
                    if thai { label_th } else { label }
                ));
            }
        }
    }
    if let Some(value) = error.get(ContextKind::ValidSubcommand) {
        rendered.push_str(&format!(
            "\n{}: {value}",
            if thai {
                "คำสั่งที่รองรับ"
            } else {
                "Commands"
            }
        ));
    }
    if let Some(usage) = error.get(ContextKind::Usage) {
        rendered.push_str(&format!("\n{usage}"));
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn runtime_errors_offer_localized_guidance_and_redact_credentials() {
        let error = anyhow::anyhow!(
            "provider failed at https://user:private-password@example.invalid?api%5Fkey=private-query"
        );
        for (language, labels) in [
            ("en", ["Cause:", "Fix:", "Next:"]),
            ("th", ["สาเหตุ:", "วิธีแก้:", "คำสั่งถัดไป:"]),
        ] {
            let rendered = format_error(&error, language);
            for label in labels {
                assert!(rendered.contains(label), "{rendered}");
            }
            assert!(rendered.contains("lbc doctor --connectivity"));
            assert!(!rendered.contains("private-password"));
            assert!(!rendered.contains("private-query"));
        }
    }

    #[test]
    fn toml_diagnostics_hide_source_snippets_and_invalid_values() {
        let error = toml::from_str::<toml::Value>("model = 'private-source-value\n").unwrap_err();
        let error = Error::new(error).context("invalid configuration");
        let rendered = format_error(&error, "en");
        assert!(rendered.contains("invalid TOML syntax or setting type"));
        assert!(rendered.contains("lbc config show"));
        assert!(!rendered.contains("private-source-value"));
        assert!(!rendered.contains("model ="));
    }

    #[test]
    fn invalid_knowledge_metadata_offers_index_recovery() {
        let rendered = format_error(
            &anyhow::anyhow!("knowledge validation failed: invalid YAML frontmatter"),
            "en",
        );
        assert!(rendered.contains("Correct the entry's YAML frontmatter metadata"));
        assert!(rendered.contains("lbc index --json; lbc doctor"));
    }

    #[test]
    fn usage_errors_keep_declared_arguments_and_hide_supplied_values() {
        for (args, expected) in [
            (vec!["lbc", "add", "--stdin"], "--title"),
            (vec!["lbc", "list", "--limit", "private-value"], "--limit"),
            (
                vec!["lbc", "setup", "--provider", "private-value"],
                "openai",
            ),
        ] {
            let error = crate::cli::args::Cli::command()
                .try_get_matches_from(args)
                .unwrap_err();
            for language in ["en", "th"] {
                let rendered = format_usage_error(&error, language);
                assert!(rendered.contains(expected), "{rendered}");
                assert!(!rendered.contains("private-value"), "{rendered}");
            }
        }
        let error = crate::cli::args::Cli::command()
            .try_get_matches_from(["lbc", "--private-value"])
            .unwrap_err();
        assert!(!format_usage_error(&error, "en").contains("private-value"));
    }
}
