use coordinate_systems::{Framed, IntoFramed, Transform};
use geometry::line_segment::LineSegment;
use nalgebra::{Isometry2, UnitComplex};
use spl_network_messages::{GamePhase, SubState};
use types::{
    coordinate_systems::{Field, Ground},
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    line::Line,
    motion_command::{HeadMotion, MotionCommand, OrientationMode},
    parameters::InterceptBallParameters,
    planned_path::PathSegment,
    step_plan::Step,
    world_state::{BallState, WorldState},
};

pub fn execute(
    world_state: &WorldState,
    parameters: InterceptBallParameters,
    maximum_step_size: Step,
) -> Option<MotionCommand> {
    if let Some(
        FilteredGameControllerState {
            game_phase: GamePhase::PenaltyShootout { .. },
            ..
        }
        | FilteredGameControllerState {
            sub_state: Some(SubState::PenaltyKick),
            ..
        },
    ) = world_state.filtered_game_controller_state
    {
        return None;
    }

    let filtered_game_state = world_state
        .filtered_game_controller_state
        .map(|filtered_game_controller_state| filtered_game_controller_state.game_state);
    match (
        filtered_game_state,
        world_state.ball,
        world_state.robot.ground_to_field,
    ) {
        (
            Some(FilteredGameState::Playing {
                ball_is_free: true, ..
            })
            | None,
            Some(ball),
            Some(ground_to_field),
        ) => {
            if !ball_is_interception_candidate(ball, ground_to_field, &parameters) {
                return None;
            }

            let Step {
                forward,
                left,
                turn: _,
            } = maximum_step_size;

            if forward == 0.0 || left == 0.0 {
                return None;
            }

            let ball_line = Line(
                ball.ball_in_ground,
                ball.ball_in_ground + ball.ball_in_ground_velocity,
            );
            let interception_point = ball_line.project_point(Framed::origin());

            if interception_point.coords().norm() > parameters.maximum_intercept_distance {
                return None;
            }

            let path = vec![PathSegment::LineSegment(LineSegment(
                Framed::origin(),
                interception_point,
            ))];

            Some(MotionCommand::Walk {
                head: HeadMotion::LookAt {
                    target: ball.ball_in_ground,
                    camera: None,
                },
                path,
                left_arm: types::motion_command::ArmMotion::Swing,
                right_arm: types::motion_command::ArmMotion::Swing,
                orientation_mode: OrientationMode::Override(UnitComplex::default().framed()),
            })
        }
        _ => None,
    }
}

fn ball_is_interception_candidate(
    ball: BallState,
    ground_to_field: Transform<Ground, Field, Isometry2<f32>>,
    parameters: &InterceptBallParameters,
) -> bool {
    let ball_is_in_front_of_robot = ball.ball_in_ground.coords().norm()
        < parameters.maximum_ball_distance
        && ball.ball_in_ground.x() > 0.0;
    let ball_is_moving_towards_robot =
        ball.ball_in_ground_velocity.x() < -parameters.minimum_ball_velocity_towards_robot;

    let ball_in_field_velocity = ground_to_field * ball.ball_in_ground_velocity;
    let ball_is_moving = ball_in_field_velocity.norm() > parameters.minimum_ball_velocity;
    let ball_is_moving_towards_own_half =
        ball_in_field_velocity.x() < -parameters.minimum_ball_velocity_towards_own_half;

    ball_is_in_front_of_robot
        && ball_is_moving
        && ball_is_moving_towards_robot
        && ball_is_moving_towards_own_half
}
