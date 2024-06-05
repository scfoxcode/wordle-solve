use std::collections::HashMap;

pub trait Distribution {
    fn charFrequency(&self, value:char) -> f32; 
    fn total(&self) -> u32;
}

pub struct FrequencyDistribution {
    mapping: HashMap<char, u32>,
    total: u32,
}

impl FrequencyDistribution {
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
            total: 0,
        }
    }

    pub fn charCount(&self, value: char) -> u32 {
        match self.mapping.get(&value) {
            Some(count) => *count,
            None => 0,
        }
    }

    pub fn incrementChar(&mut self, value: char) {
        let count = self.mapping.entry(value).or_insert(0);
        *count += 1;
        self.total += 1;
    }
}

impl Distribution for FrequencyDistribution {
    fn charFrequency(&self, value: char) -> f32 {
        if self.total < 1 {
            return 0.0; // Avoid divide by zero later
        }
        self.charCount(value) as f32 / self.total as f32
    }

    fn total(&self) -> u32 {
        self.total
    }
}

pub struct FrequencySet {
    distributions: [FrequencyDistribution; 5]
}

impl FrequencySet {
    pub fn new() -> Self {
        Self { // only done this way because my hashmap does not implement copy
            distributions: [
                FrequencyDistribution::new(),
                FrequencyDistribution::new(),
                FrequencyDistribution::new(),
                FrequencyDistribution::new(),
                FrequencyDistribution::new(),
            ],
        }
    }

    pub fn distributionForIndex(&mut self, index: usize) -> Result<&mut FrequencyDistribution, &'static str> {
        if index >= self.distributions.len() {
            return Err("Error. Asked for distribution that is out of range");
        }
        Ok(&mut self.distributions[index])
    }

    pub fn buildSetFromWords(&mut self, words: &Vec<&str>) {
        for word in words.iter() {
            for (i, letter) in word.chars().enumerate() {
                if i > 4 {
                    panic!("Fatal error. Attempted to index beyond 5th letter");
                }
                // How does rust handle bounds checking in these situations
                // Seems like it doesn't detect that index could be out of range. Impossible to
                // determine at compile time I suppose
                self.distributions[i].incrementChar(letter);
            }
        }
    }
}

impl Distribution for FrequencySet {
    fn charFrequency(&self, value: char) -> f32 {
        let count = self.distributions.iter().fold(0, |total, dist| total + dist.charCount(value));
        let total = self.total();

        if total < 1 {
            return 0.0; // Avoid divide by zero later
        }

        count as f32 / total as f32
    }

    fn total(&self) -> u32 {
        self.distributions.iter().fold(0, |total, dist| total + dist.total())
    }
}

