use chrono::Duration;
use nalgebra::{Translation2, Translation3};
use types::{LineSegment, PrimaryState};

use crate::robot::Robot;

pub struct State {
    pub time_elapsed: Duration,
    pub robots: Vec<Robot>,
}

impl State {
    pub fn new(robot_count: usize) -> Self {
        let robots: Vec<_> = (0..robot_count).map(|index| Robot::new(index)).collect();

        Self {
            time_elapsed: Duration::zero(),
            robots,
        }
    }

    pub fn stiffen_robots(&mut self) {
        for robot in &mut self.robots {
            robot.database.main_outputs.primary_state = PrimaryState::Playing;
        }
    }

    pub fn cycle(&mut self) {
        for robot in &mut self.robots {
            println!("cycling");
            robot.cycle().unwrap();
            let database = robot.database.clone();
            println!("{:?}", database.main_outputs.motion_command);
            println!(
                "{:?}",
                database.main_outputs.robot_to_field.unwrap().translation
            );
            match database.main_outputs.motion_command {
                types::MotionCommand::Walk {
                    head,
                    path,
                    left_arm,
                    right_arm,
                    orientation_mode,
                } => {
                    if let Some(robot_to_field) =
                        robot.database.main_outputs.robot_to_field.as_mut()
                    {
                        let position = match path[0] {
                            types::PathSegment::LineSegment(LineSegment(start, end)) => {
                                println!("{:?}", path);
                                println!("{:?}", start);
                                println!("{:?}", end);
                                end
                            }
                            types::PathSegment::Arc(arc, _orientation) => arc.end,
                        } * 0.5;
                        println!("{:?}", position);
                        robot_to_field
                            .append_translation_mut(&Translation2::new(position.x, position.y));
                    }
                }
                types::MotionCommand::InWalkKick {
                    head,
                    kick,
                    kicking_side,
                } => todo!(),
                _ => {}
            }
        }
    }
}