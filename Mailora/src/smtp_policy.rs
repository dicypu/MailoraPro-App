// SMTP Submission Policy: MAIL FROM kimliği doğrulama
pub fn is_mail_from_allowed(user_id: &str, mail_from: &str, allowed_identities: &[String]) -> bool {
    allowed_identities.contains(&mail_from.to_string())
}
