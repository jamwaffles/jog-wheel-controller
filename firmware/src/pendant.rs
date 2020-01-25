use crate::multiplier::SelectedMultiplier;

#[derive(Debug, Copy, Clone)]
pub struct Pendant {
    is_estopped: bool,
    multiplier: Option<SelectedMultiplier>,
}

impl Pendant {
    pub fn new() -> Self {
        Self {
            is_estopped: true,
            multiplier: None,
        }
    }

    pub fn set_estop(&mut self) {
        self.is_estopped = true;
    }

    pub fn clear_estop(&mut self) {
        self.is_estopped = false;
    }

    pub fn set_multiplier(&mut self, multiplier: Option<SelectedMultiplier>) {
        self.multiplier = multiplier
    }

    pub fn estopped(&self) -> bool {
        self.is_estopped
    }

    pub fn multiplier(&self) -> Option<SelectedMultiplier> {
        self.multiplier
    }
}
