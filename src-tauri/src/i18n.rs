//! Backend translations: tray menu, tooltip and notifications. The UI has its
//! own dictionaries (src/lib/i18n); user-facing errors are sent to it as codes.

use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Pl,
}

impl Lang {
    /// Resolves the stored preference ("auto", "en", "pl").
    pub fn from_pref(pref: &str) -> Self {
        match pref {
            "en" => Lang::En,
            "pl" => Lang::Pl,
            _ => Self::system(),
        }
    }

    fn system() -> Self {
        match sys_locale::get_locale() {
            Some(locale) if locale.to_lowercase().starts_with("pl") => Lang::Pl,
            _ => Lang::En,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Text {
    NoMouse,
    TooltipNoMouse,
    Open,
    StartWithSystem,
    Quit,
    Profile,
    /// "{name} profile"
    ProfileOf,
    Reapply,
    Asleep,
    Connected,
    LowBatteryTitle,
    /// "{name}: {level}% left"
    LowBatteryBody,
}

pub fn tr(lang: Lang, text: Text) -> &'static str {
    use Text::*;
    match lang {
        Lang::En => match text {
            NoMouse => "No mouse detected",
            TooltipNoMouse => "RivalGUI - no mouse detected",
            Open => "Open RivalGUI",
            StartWithSystem => "Start with system",
            Quit => "Quit",
            Profile => "Profile",
            ProfileOf => "{name} profile",
            Reapply => "Re-apply settings",
            Asleep => "asleep",
            Connected => "connected",
            LowBatteryTitle => "Low mouse battery",
            LowBatteryBody => "{name}: {level}% left",
        },
        Lang::Pl => match text {
            NoMouse => "Nie wykryto myszy",
            TooltipNoMouse => "RivalGUI - nie wykryto myszy",
            Open => "Otwórz RivalGUI",
            StartWithSystem => "Uruchamiaj z systemem",
            Quit => "Zakończ",
            Profile => "Profil",
            ProfileOf => "Profil: {name}",
            Reapply => "Zastosuj ponownie",
            Asleep => "uśpiona",
            Connected => "podłączona",
            LowBatteryTitle => "Niski poziom baterii myszy",
            LowBatteryBody => "{name}: pozostało {level}%",
        },
    }
}

/// Translates the connection mode suffix of rivalcfg device names, e.g. "(wired mode)".
pub fn device_name(lang: Lang, name: &str) -> String {
    match lang {
        Lang::En => name.to_string(),
        Lang::Pl => name
            .replace("(wired mode)", "(tryb przewodowy)")
            .replace("(2.4 GHz wireless mode)", "(tryb bezprzewodowy 2,4 GHz)")
            .replace("(2.4 GHz mode)", "(tryb 2,4 GHz)"),
    }
}

/// Display name of a profile: the built-in "Default" profile is translated.
pub fn profile_name(lang: Lang, name: &str) -> String {
    match (lang, name) {
        (Lang::Pl, "Default") => "Domyślny".into(),
        _ => name.to_string(),
    }
}

/// A user-facing error the UI translates: `@code|arg` (see `err_*` keys in src/lib/i18n/en.ts).
pub fn coded(code: &str, arg: impl Display) -> String {
    format!("@{code}|{arg}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefs_and_codes() {
        assert_eq!(Lang::from_pref("pl"), Lang::Pl);
        assert_eq!(Lang::from_pref("en"), Lang::En);
        assert_eq!(coded("profile_exists", "Gaming"), "@profile_exists|Gaming");
        assert_eq!(tr(Lang::Pl, Text::Quit), "Zakończ");
    }
}
