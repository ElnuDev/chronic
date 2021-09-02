use const_format::concatcp;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;
use std::{
    fs::File,
    io::{stdin, BufRead, BufReader},
    path::Path,
};
use uuid::Uuid;

const HABITCTL_DIR: &str = "/home/elnu/.habitctl";
const HABITCTL_HABITS: &str = concatcp!(HABITCTL_DIR, "/habits");
const HABITCTL_LOG: &str = concatcp!(HABITCTL_DIR, "/log");

const DIR: &str = "/home/elnu/.chronic";
const HABITS: &str = concatcp!(DIR, "/habits");
const LOG: &str = concatcp!(DIR, "/log");

const NAME: &str = "chronic";

fn main() {
    if !installed() {
        setup();
    }
}

fn setup() {
    println!("Welcome to {}!", NAME);
    if habitctl_installed() {
        println!(
            "A habitctl installation has been detected in {}",
            HABITCTL_DIR
        );
        println!("Would you like to import it into {}? (Y/n)", NAME);
        let import = loop {
            let mut input = String::new();
            stdin().read_line(&mut input).unwrap();
            input = input.trim().to_lowercase();
            if input.is_empty() || input == "y" {
                break true;
            } else if input == "n" {
                break false;
            }
            println!("Invalid response.");
        };
        if import {
            let habits = parse_habitctl_data();
            println!("{}", serde_yaml::to_string(&habits).unwrap());
        }
    }
}

fn installed() -> bool {
    Path::new(HABITS).is_file() && Path::new(LOG).is_file()
}

fn habitctl_installed() -> bool {
    Path::new(HABITCTL_HABITS).is_file() && Path::new(HABITCTL_LOG).is_file()
}

fn parse_habitctl_data() -> Vec<Habit> {
    let habits = File::open(HABITCTL_HABITS).unwrap();
    let habits_reader = BufReader::new(habits);
    let mut habit_list = Vec::<Habit>::new();
    for line in habits_reader.lines().flatten() {
        let habit = Habit::from_habitctl_line(&line);
        if habit.is_none() {
            continue;
        }
        let habit = habit.unwrap();
        habit_list.push(habit);
    }
    habit_list
}

#[derive(Debug, Serialize, Deserialize)]
struct Habit {
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    uuid: Uuid,
    r#type: HabitType,
    description: String,
}

fn serialize_uuid<S>(uuid: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_str(&format_args!("{:?}", uuid))
}

fn deserialize_uuid<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
where
    D: Deserializer<'de>,
{
    struct UuidVisitor;

    impl<'de> Visitor<'de> for UuidVisitor {
        type Value = Uuid;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string containing a UUID")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Uuid::from_str(value).unwrap())
        }
    }

    deserializer.deserialize_any(UuidVisitor)
}

impl Habit {
    fn new(r#type: HabitType, description: String) -> Self {
        Self {
            r#type,
            description,
            uuid: Uuid::new_v4(),
        }
    }

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
        Some(Self::new(habit_type, description))
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum HabitType {
    #[serde(rename = "just_track")]
    JustTrack,

    #[serde(rename = "daily")]
    Daily,

    #[serde(rename = "weekly")]
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
