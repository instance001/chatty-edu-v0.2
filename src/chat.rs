use crate::local_model;
use crate::settings::{JanetConfig, Settings};

pub fn generate_answer(settings: &Settings, user_input: &str) -> String {
    match local_model::chat_completion(&settings.model, user_input) {
        Ok(text) => text,
        Err(err) => format!("I couldn't run the local model yet ({err})."),
    }
}

pub fn janet_filter(janet: &JanetConfig, answer: &str, user_input: &str) -> String {
    if !janet.enabled {
        return answer.to_string();
    }

    let banned_swears = [
        "fuck", "shit", "cunt", "bitch", "bastard", "crap", "piss", "dick", "cock", "tits",
        "asshole", "ass", "bollock",
    ];
    let masked_swears = ["fk", "fck", "fuk", "sht", "sh1t", "btch", "b1tch", "biatch"];
    let banned_mature = ["sex", "porn", "drugs", "suicide", "kill", "terrorist"];

    let normalize = |text: &str| -> String {
        text.to_lowercase()
            .chars()
            .filter_map(|c| match c {
                '0' => Some('o'),
                '1' | '!' | '|' => Some('i'),
                '3' => Some('e'),
                '4' => Some('a'),
                '5' => Some('s'),
                '7' => Some('t'),
                '8' => Some('b'),
                '9' => Some('g'),
                _ if c.is_ascii_alphabetic() => Some(c),
                _ => None, // strip masking like *, -, _
            })
            .collect()
    };
    let drop_vowels = |text: &str| -> String {
        text.chars()
            .filter(|c| !matches!(c, 'a' | 'e' | 'i' | 'o' | 'u'))
            .collect()
    };

    let lower_in = user_input.to_lowercase();
    let lower_ans = answer.to_lowercase();
    let normalized_in = normalize(&lower_in);
    let _normalized_ans = normalize(&lower_ans);
    let vowelless_in = drop_vowels(&normalized_in);

    let contains_swear = janet.block_swears
        && banned_swears
            .iter()
            .any(|w| {
                let w_vowelless = drop_vowels(w);
                lower_in.contains(w)
                    || normalized_in.contains(w)
                    || (!w_vowelless.is_empty() && vowelless_in.contains(&w_vowelless))
            });

    let masked_hit = janet.block_swears
        && masked_swears
            .iter()
            .any(|w| normalized_in.contains(w));

    let contains_mature = janet.block_mature_topics
        && banned_mature
            .iter()
            .any(|w| lower_in.contains(w));

    if contains_swear || masked_hit || contains_mature {
        return janet
            .fallback_message
            .clone();
    }

    answer.to_string()
}
