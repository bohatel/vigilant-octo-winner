use once_cell::sync::Lazy;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use regex::Regex;

static TOKEN_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^A-Za-z0-9]").unwrap());

#[derive(Debug)]
pub struct SubscriptionToken(String);

impl SubscriptionToken {
    pub fn generate() -> SubscriptionToken {
        let mut rng = thread_rng();
        let s = std::iter::repeat_with(|| rng.sample(Alphanumeric))
            .map(char::from)
            .take(30)
            .collect();
        Self(s)
    }

    pub fn parse(s: String) -> Result<SubscriptionToken, String> {
        let is_empty = s.is_empty();
        let is_too_long = s.len() > 30;
        let contains_forbiden_characters = TOKEN_REGEX.is_match(&s);

        if is_empty || is_too_long || contains_forbiden_characters {
            Err(format!("{s} is not a valid confirmation token."))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for SubscriptionToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriptionToken;
    use claims::{assert_err, assert_ok};
    use proptest::prelude::*;

    #[test]
    fn generate_produces_valid_token() {
        let token = SubscriptionToken::generate();

        let new_token = SubscriptionToken::parse(token.as_ref().into());
        assert_ok!(new_token);
    }

    #[test]
    fn bad_tokens_are_rejected() {
        // token is too long
        let s = "q".repeat(31);
        let token = SubscriptionToken::parse(s);
        assert_err!(token);

        // token is empty
        let token = SubscriptionToken::parse("".into());
        assert_err!(token);
    }

    proptest! {
        #[test]
        fn invalid_chars_are_rejected(mut s in "[A-Za-z0-9]{29}[^A-Za-z0-9]{1}"){
            if s.len() > 30 {
                s = s[s.len() - 30..].to_string();
            }
            dbg!(&s);
            assert_eq!(s.len(), 30);
            let token = SubscriptionToken::parse(s.into());
            assert_err!(token);
        }
    }
}
