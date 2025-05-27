pub enum ReceiveError {
    Weak,
    NotEnough,
    NoValid,
}

impl ReceiveError {
    pub fn to_string(&self) -> String {
        match self {
            ReceiveError::Weak => "Signal too weak".to_string(),
            ReceiveError::NotEnough => "Too few samples".to_string(),
            ReceiveError::NoValid => "No valid text decoded".to_string(),
        }
    }
}
