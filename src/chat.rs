use crate::settings::JanetConfig;

pub fn generate_answer_stub(user_input: &str) -> String {
    format!(
        "This is a placeholder answer for: \"{}\".\nOnce the model is wired, I'll explain this properly.",
        user_input
    )
}

pub fn janet_filter(janet: &JanetConfig, answer: &str, user_input: &str) -> String {
    if !janet.enabled {
        return answer.to_string();
    }

    let banned_swears = ["fuck", "shit", "cunt", "bitch", "bastard"];
    let banned_mature = ["sex", "porn", "drugs", "suicide", "kill", "terrorist"];

    let lower_in = user_input.to_lowercase();
    let lower_ans = answer.to_lowercase();

    let contains_swear = janet.block_swears
        && banned_swears
            .iter()
            .any(|w| lower_in.contains(w) || lower_ans.contains(w));

    let contains_mature = janet.block_mature_topics
        && banned_mature
            .iter()
            .any(|w| lower_in.contains(w) || lower_ans.contains(w));

    if contains_swear || contains_mature {
        janet.fallback_message.clone()
    } else {
        answer.to_string()
    }
}
