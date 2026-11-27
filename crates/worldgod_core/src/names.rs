//! Генератор названий стран и поселений.

use crate::rng::Rng;

const FIRST: [&str; 18] = [
    "Аль", "Вел", "Гор", "Дра", "Зла", "Ир", "Кар", "Лум", "Мор", "Нор", "Ор", "Ро", "Скай", "Тир",
    "Уль", "Яр", "Бер", "Фал",
];

const MIDDLE: [&str; 12] = [
    "а", "о", "и", "е", "у", "ан", "ор", "ел", "ин", "ур", "ав", "ис",
];

const LAST: [&str; 14] = [
    "ния", "дор", "мир", "град", "ланд", "рия", "тана", "вент", "холм", "стан", "элия", "ум",
    "ар", "ол",
];

/// Придумывает название страны, не совпадающее с уже занятыми.
pub fn nation_name(rng: &mut Rng, taken: &[String]) -> String {
    for _ in 0..64 {
        let name = format!(
            "{}{}{}",
            rng.pick(&FIRST),
            rng.pick(&MIDDLE),
            rng.pick(&LAST)
        );
        let capitalized = capitalize(&name);
        if !taken.iter().any(|existing| *existing == capitalized) {
            return capitalized;
        }
    }

    // Крайне маловероятный случай: добавляем номер.
    format!("{} {}", capitalize(&random_word(rng)), taken.len() + 1)
}

/// Название поселения (дом, форпост).
pub fn settlement_name(rng: &mut Rng) -> String {
    format!(
        "{}{}",
        rng.pick(&FIRST),
        rng.pick(&["овка", "ино", "ово", "ец", "ы", "ск"])
    )
}

fn random_word(rng: &mut Rng) -> String {
    format!("{}{}{}", rng.pick(&FIRST), rng.pick(&MIDDLE), rng.pick(&LAST))
}

/// Делает первую букву заглавной (работает с кириллицей UTF-8).
pub fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_and_capitalized() {
        let mut rng = Rng::new(3);
        let mut taken: Vec<String> = Vec::new();
        for _ in 0..40 {
            let name = nation_name(&mut rng, &taken);
            assert!(!name.is_empty());
            assert!(!taken.contains(&name), "повтор названия: {name}");
            let first = name.chars().next().unwrap();
            assert!(first.is_uppercase(), "название не с заглавной: {name}");
            taken.push(name);
        }
    }

    #[test]
    fn capitalize_works() {
        assert_eq!(capitalize("альдор"), "Альдор");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn settlements_have_suffix() {
        let mut rng = Rng::new(11);
        let name = settlement_name(&mut rng);
        assert!(name.chars().next().unwrap().is_uppercase());
        assert!(name.len() > 3);
    }
}
