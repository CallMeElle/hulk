use std::ops::Range;

use coordinate_systems::{distance, Framed, IntoFramed, IntoTransform, Transform};
use framework::AdditionalOutput;
use geometry::look_at::LookAt;
use nalgebra::{point, vector, Isometry2, Point2};
use spl_network_messages::{GamePhase, SubState, Team};
use types::{
    coordinate_systems::{Field, Ground},
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    line::Line,
    motion_command::MotionCommand,
    parameters::RolePositionsParameters,
    path_obstacles::PathObstacle,
    support_foot::Side,
    world_state::{BallState, WorldState},
};

use super::{head::LookAction, walk_to_pose::WalkAndStand};

pub struct Defend<'cycle> {
    world_state: &'cycle WorldState,
    field_dimensions: &'cycle FieldDimensions,
    role_positions: &'cycle RolePositionsParameters,
    walk_and_stand: &'cycle WalkAndStand<'cycle>,
    look_action: &'cycle LookAction<'cycle>,
}

impl<'cycle> Defend<'cycle> {
    pub fn new(
        world_state: &'cycle WorldState,
        field_dimensions: &'cycle FieldDimensions,
        role_positions: &'cycle RolePositionsParameters,
        walk_and_stand: &'cycle WalkAndStand,
        look_action: &'cycle LookAction,
    ) -> Self {
        Self {
            world_state,
            field_dimensions,
            role_positions,
            walk_and_stand,
            look_action,
        }
    }

    fn with_pose(
        &self,
        pose: Transform<Ground, Ground, Isometry2<f32>>,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Option<MotionCommand> {
        self.walk_and_stand
            .execute(pose, self.look_action.execute(), path_obstacles_output)
    }

    pub fn left(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Option<MotionCommand> {
        let pose = defend_left_pose(self.world_state, self.field_dimensions, self.role_positions)?;
        self.with_pose(pose, path_obstacles_output)
    }

    pub fn right(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Option<MotionCommand> {
        let pose = defend_right_pose(self.world_state, self.field_dimensions, self.role_positions)?;
        self.with_pose(pose, path_obstacles_output)
    }

    pub fn penalty_kick(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Option<MotionCommand> {
        let pose =
            defend_penalty_kick(self.world_state, self.field_dimensions, self.role_positions)?;
        self.with_pose(pose, path_obstacles_output)
    }

    pub fn goal(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Option<MotionCommand> {
        let pose = defend_goal_pose(self.world_state, self.field_dimensions, self.role_positions)?;
        self.with_pose(pose, path_obstacles_output)
    }

    pub fn kick_off(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Option<MotionCommand> {
        let pose =
            defend_kick_off_pose(self.world_state, self.field_dimensions, self.role_positions)?;
        self.with_pose(pose, path_obstacles_output)
    }
}

fn defend_left_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositionsParameters,
) -> Option<Transform<Ground, Ground, Isometry2<f32>>> {
    let ground_to_field = world_state.robot.ground_to_field?;
    let ball = world_state
        .rule_ball
        .or(world_state.ball)
        .unwrap_or_else(|| BallState::new_at_center(ground_to_field));

    let position_to_defend = point![
        -field_dimensions.length / 2.0,
        role_positions.defender_y_offset
    ]
    .framed();
    let mut distance_to_target = if ball.field_side == Side::Left {
        role_positions.defender_aggressive_ring_radius
    } else {
        role_positions.defender_passive_ring_radius
    };
    distance_to_target = penalty_kick_defender_radius(
        distance_to_target,
        world_state.filtered_game_controller_state,
        field_dimensions,
    );
    let defend_pose = block_on_circle(ball.ball_in_field, position_to_defend, distance_to_target);
    let field_to_ground = ground_to_field.inverse();
    Some(field_to_ground * defend_pose)
}

fn defend_right_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositionsParameters,
) -> Option<Transform<Ground, Ground, Isometry2<f32>>> {
    let ground_to_field = world_state.robot.ground_to_field?;
    let ball = world_state
        .rule_ball
        .or(world_state.ball)
        .unwrap_or_else(|| BallState::new_at_center(ground_to_field));

    let position_to_defend = point![
        -field_dimensions.length / 2.0,
        -role_positions.defender_y_offset
    ]
    .framed();
    let mut distance_to_target = if ball.field_side == Side::Right {
        role_positions.defender_aggressive_ring_radius
    } else {
        role_positions.defender_passive_ring_radius
    };
    distance_to_target = penalty_kick_defender_radius(
        distance_to_target,
        world_state.filtered_game_controller_state,
        field_dimensions,
    );
    let defend_pose = block_on_circle(ball.ball_in_field, position_to_defend, distance_to_target);
    Some(ground_to_field.inverse() * defend_pose)
}

fn defend_penalty_kick(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositionsParameters,
) -> Option<Transform<Ground, Ground, Isometry2<f32>>> {
    let ground_to_field = world_state.robot.ground_to_field?;
    let ball = world_state
        .rule_ball
        .or(world_state.ball)
        .unwrap_or_else(|| BallState::new_at_center(ground_to_field));

    let position_to_defend = point![
        (-field_dimensions.length + field_dimensions.penalty_area_length) / 2.0,
        0.0
    ]
    .framed();
    let mut distance_to_target = if ball.field_side == Side::Left {
        role_positions.defender_aggressive_ring_radius
    } else {
        role_positions.defender_passive_ring_radius
    };
    distance_to_target = penalty_kick_defender_radius(
        distance_to_target,
        world_state.filtered_game_controller_state,
        field_dimensions,
    );

    let defend_pose = block_on_circle(ball.ball_in_field, position_to_defend, distance_to_target);
    Some(ground_to_field.inverse() * defend_pose)
}

fn defend_goal_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositionsParameters,
) -> Option<Transform<Ground, Ground, Isometry2<f32>>> {
    let ground_to_field = world_state.robot.ground_to_field?;
    let ball = world_state
        .rule_ball
        .or(world_state.ball)
        .unwrap_or_else(|| BallState::new_at_center(ground_to_field));

    let keeper_x_offset = match world_state.filtered_game_controller_state {
        Some(
            FilteredGameControllerState {
                game_phase:
                    GamePhase::PenaltyShootout {
                        kicking_team: Team::Opponent,
                    },
                ..
            }
            | FilteredGameControllerState {
                sub_state: Some(SubState::PenaltyKick),
                kicking_team: Team::Opponent,
                ..
            },
        ) => 0.0,
        _ => role_positions.keeper_x_offset,
    };

    let position_to_defend = point![-field_dimensions.length / 2.0 - 1.0, 0.0].framed();
    let defend_pose = block_on_line(
        ball.ball_in_field,
        position_to_defend,
        -field_dimensions.length / 2.0 + keeper_x_offset,
        -0.7..0.7,
    );
    Some(ground_to_field.inverse() * defend_pose)
}

fn defend_kick_off_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositionsParameters,
) -> Option<Transform<Ground, Ground, Isometry2<f32>>> {
    let ground_to_field = world_state.robot.ground_to_field?;
    let absolute_ball_position = match world_state.ball {
        Some(ball) => ball.ball_in_field,
        None => Point2::origin().framed(),
    };
    let position_to_defend = point![-field_dimensions.length / 2.0, 0.0].framed();
    let center_circle_radius = field_dimensions.center_circle_diameter / 2.0;
    let distance_to_target = distance(&position_to_defend, &absolute_ball_position)
        - center_circle_radius
        - role_positions.striker_distance_to_non_free_center_circle;
    let defend_pose = block_on_circle(
        absolute_ball_position,
        position_to_defend,
        distance_to_target,
    );
    Some(ground_to_field.inverse() * defend_pose)
}

pub fn block_on_circle(
    ball_position: Framed<Field, Point2<f32>>,
    target: Framed<Field, Point2<f32>>,
    distance_to_target: f32,
) -> Transform<Ground, Field, Isometry2<f32>> {
    let target_to_ball = ball_position - target;
    let block_position = target + (target_to_ball.inner.normalize() * distance_to_target).framed();
    Isometry2::new(
        block_position.inner.coords,
        block_position.inner.look_at(&ball_position.inner).angle(),
    )
    .framed_transform()
}

fn block_on_line(
    ball_position: Framed<Field, Point2<f32>>,
    target: Framed<Field, Point2<f32>>,
    defense_line_x: f32,
    defense_line_y_range: Range<f32>,
) -> Transform<Ground, Field, Isometry2<f32>> {
    let is_ball_in_front_of_defense_line = defense_line_x < ball_position.x();
    if is_ball_in_front_of_defense_line {
        let defense_line = Line(
            point![defense_line_x, defense_line_y_range.start].framed(),
            point![defense_line_x, defense_line_y_range.end].framed(),
        );
        let ball_target_line = Line(ball_position, target);
        let intersection_point = defense_line.intersection(&ball_target_line);
        let defense_position = point![
            intersection_point.x(),
            intersection_point
                .y()
                .clamp(defense_line_y_range.start, defense_line_y_range.end)
        ]
        .framed();
        Isometry2::new(
            defense_position.inner.coords,
            defense_position.look_at(&ball_position).inner.angle(),
        )
        .framed_transform()
    } else {
        let defense_position = point![
            defense_line_x,
            (defense_line_y_range.start + defense_line_y_range.end) / 2.0
        ]
        .framed();
        Isometry2::new(
            defense_position.inner.coords,
            defense_position.look_at(&ball_position).inner.angle(),
        )
        .framed_transform()
    }
}

fn penalty_kick_defender_radius(
    distance_to_target: f32,
    filtered_game_controller_state: Option<FilteredGameControllerState>,
    field_dimensions: &FieldDimensions,
) -> f32 {
    if let Some(FilteredGameControllerState {
        kicking_team: Team::Opponent,
        sub_state: Some(SubState::PenaltyKick),
        ..
    }) = filtered_game_controller_state
    {
        let half_penalty_width = field_dimensions.penalty_area_width / 2.0;
        let minimum_penalty_defender_radius =
            vector![field_dimensions.penalty_area_length, half_penalty_width].norm();
        distance_to_target.max(minimum_penalty_defender_radius)
    } else {
        distance_to_target
    }
}
