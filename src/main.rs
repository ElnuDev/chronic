use const_format::concatcp;
use std::io::BufRead;
use std::{fs::File, io::BufReader, path::Path};

const HABITCTL_DIR: &str = "/home/elnu/.habitctl";
const HABITCTL_HABITS: &str = concatcp!(HABITCTL_DIR, "/habits");
const HABITCTL_LOG: &str = concatcp!(HABITCTL_DIR, "/log");

fn main() {
    if habitctl_installed() {
        println!("habitctl is installed.");
        parse_habitctl_data();
    } else {
        println!("habitctl isn't installed.");
    }
}

fn habitctl_installed() -> bool {
    Path::new(HABITCTL_HABITS).is_file() && Path::new(HABITCTL_LOG).is_file()
}

fn parse_habitctl_data() {
    let habits = File::open(HABITCTL_HABITS).unwrap();
    let habits_reader = BufReader::new(habits);

    for line in habits_reader.lines().flatten() {
        let habit = Habit::from_habitctl_line(&line);
        if habit.is_none() {
            continue;
        }
        let habit = habit.unwrap();
        println!("{:?}", habit)
    }
}

#[derive(Debug)]
struct Habit {
    r#type: HabitType,
    description: String,
}

impl Habit {
    fn from_habitctl_line(line: &str) -> Option<Self> {
        let line = line.trim();
        // Immediately exit out if the line is empty after trimming,
        // since the next line.chars().next().unwrap() will panic if it is empty
        if line.is_empty() {
            return None;
        }
        let habit_type = HabitType::from_habitctl_char(line.chars().next().unwrap());
        // Return None if HabitType::from_char returns a None variant.
        // Otherwise, unwrap.
        let habit_type = habit_type?;
        // Remove the first character that habitctl uses for type distinction.
        let description = line[1..].trim().to_string();
        Some(Habit {
            r#type: habit_type,
            description,
        })
    }
}

#[derive(Clone, Copy, Debug)]
enum HabitType {
    JustTrack,
    Daily,
    Weekly,
}

impl HabitType {
    fn from_habitctl_char(char: char) -> Option<Self> {
        match char {
            '0' => Some(Self::JustTrack),
            '1' => Some(Self::Daily),
            '7' => Some(Self::Weekly),
            _ => None,
        }
    }
}
