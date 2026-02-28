use std::panic;
use models::enums::class::JobName;
use models::enums::{EnumWithNumberValue, EnumWithStringValue};

pub fn job_class_name(class: u16) -> String {
    let result = panic::catch_unwind(|| {
        let job = JobName::try_from_value(class as usize);
        job.map(|j| j.as_str().to_string())
    });
    match result {
        Ok(Ok(name)) => name,
        _ => "Unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_classes() {
        assert_eq!(job_class_name(0), "Novice");
        assert_eq!(job_class_name(7), "Knight");
        assert_eq!(job_class_name(4008), "LordKnight");
        assert_eq!(job_class_name(4013), "AssassinCross");
    }

    #[test]
    fn unknown_class() {
        assert_eq!(job_class_name(9999), "Unknown");
    }
}
