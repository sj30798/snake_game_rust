use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::{cursor, execute};
use rand::Rng;
use std::io::{self, Write};
use std::time::Duration;

const WIDTH: i32 = 20;
const HEIGHT: i32 = 20;
const TICK_MS: u64 = 200;
const STATE_BITS: usize = 11;
const STATE_COUNT: usize = 1 << STATE_BITS;

type Point = (i32, i32);

enum StepOutcome {
    Continue,
    Quit,
}

pub enum ControlMode {
    Manual,
    Auto { episodes: usize },
}

#[derive(Copy, Clone)]
enum TurnAction {
    Straight,
    Left,
    Right,
}

const ACTIONS: [TurnAction; 3] = [TurnAction::Straight, TurnAction::Left, TurnAction::Right];

struct StepResult {
    outcome: StepOutcome,
    reward: f32,
}

struct RlAgent {
    q_values: Vec<[f32; 3]>,
    learning_rate: f32,
    discount: f32,
    epsilon: f32,
    min_epsilon: f32,
    epsilon_decay: f32,
}

impl RlAgent {
    fn new() -> Self {
        Self {
            q_values: vec![[0.0; 3]; STATE_COUNT],
            learning_rate: 0.1,
            discount: 0.95,
            epsilon: 1.0,
            min_epsilon: 0.05,
            epsilon_decay: 0.999,
        }
    }

    fn best_action_index(&self, state: usize) -> usize {
        let values = self.q_values[state];
        let mut best_index = 0;
        let mut best_value = values[0];

        for (idx, value) in values.iter().enumerate().skip(1) {
            if *value > best_value {
                best_value = *value;
                best_index = idx;
            }
        }

        best_index
    }

    fn select_action_index(&self, state: usize, explore: bool) -> usize {
        if explore && rand::random::<f32>() < self.epsilon {
            rand::thread_rng().gen_range(0..ACTIONS.len())
        } else {
            self.best_action_index(state)
        }
    }

    fn update(&mut self, state: usize, action_index: usize, reward: f32, next_state: usize, done: bool) {
        let next_max = if done {
            0.0
        } else {
            self.q_values[next_state]
                .iter()
                .copied()
                .fold(f32::NEG_INFINITY, f32::max)
        };

        let target = reward + self.discount * next_max;
        let current = self.q_values[state][action_index];
        self.q_values[state][action_index] = current + self.learning_rate * (target - current);
    }

    fn decay_exploration(&mut self) {
        self.epsilon = (self.epsilon * self.epsilon_decay).max(self.min_epsilon);
    }
}

pub struct Game {
    snake: Vec<Point>,
    direction: Point,
    next_direction: Point,
    food: Point,
    score: u32,
    steps_since_food: u32,
    mode: ControlMode,
    agent: Option<RlAgent>,
}

impl Game {
    pub fn new(mode: ControlMode) -> Self {
        let snake = vec![(5, 5)];
        let food = spawn_food(&snake);

        Self {
            snake,
            direction: (1, 0),
            next_direction: (1, 0),
            food,
            score: 0,
            steps_since_food: 0,
            mode,
            agent: None,
        }
    }

    pub fn prepare_auto(&mut self) {
        if let ControlMode::Auto { episodes } = self.mode {
            self.agent = Some(self.train_agent(episodes));
            self.reset();
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        let mut stdout = io::stdout();

        loop {
            if self.handle_inputs()? {
                break;
            }

            if matches!(self.mode, ControlMode::Auto { .. }) {
                self.apply_agent_decision();
            }

            if let StepOutcome::Quit = self.step().outcome {
                break;
            }

            self.draw(&mut stdout)?;
            std::thread::sleep(Duration::from_millis(TICK_MS));
        }

        println!("\nGame over! Final score: {}", self.score);
        Ok(())
    }

    fn handle_inputs(&mut self) -> io::Result<bool> {
        while event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                    return Ok(true);
                }

                if matches!(self.mode, ControlMode::Auto { .. }) {
                    continue;
                }

                match key.code {
                    KeyCode::Up if self.direction != (0, 1) => self.next_direction = (0, -1),
                    KeyCode::Down if self.direction != (0, -1) => self.next_direction = (0, 1),
                    KeyCode::Left if self.direction != (1, 0) => self.next_direction = (-1, 0),
                    KeyCode::Right if self.direction != (-1, 0) => self.next_direction = (1, 0),
                    _ => {}
                }
            }
        }

        Ok(false)
    }

    fn step(&mut self) -> StepResult {
        self.direction = self.next_direction;
        let new_head = self.next_head();
        let old_head = self.snake[0];

        if self.hit_wall(new_head) || self.hit_self(new_head) {
            return StepResult {
                outcome: StepOutcome::Quit,
                reward: -10.0,
            };
        }

        self.snake.insert(0, new_head);

        if new_head == self.food {
            self.score += 1;
            self.steps_since_food = 0;
            self.food = spawn_food(&self.snake);
            StepResult {
                outcome: StepOutcome::Continue,
                reward: 10.0,
            }
        } else {
            self.snake.pop();
            self.steps_since_food += 1;

            let old_distance = manhattan_distance(old_head, self.food);
            let new_distance = manhattan_distance(new_head, self.food);

            if self.steps_since_food > (WIDTH * HEIGHT * 2) as u32 {
                return StepResult {
                    outcome: StepOutcome::Quit,
                    reward: -10.0,
                };
            }

            StepResult {
                outcome: StepOutcome::Continue,
                reward: if new_distance < old_distance { 0.15 } else { -0.2 },
            }
        }
    }

    fn draw(&self, stdout: &mut io::Stdout) -> io::Result<()> {
        execute!(stdout, cursor::MoveTo(0, 0))?;

        let border = "+".to_string() + &"-".repeat(WIDTH as usize) + "+";
        println!("{}\r", border);

        for y in 0..HEIGHT {
            print!("|");
            for x in 0..WIDTH {
                if self.snake[0] == (x, y) {
                    print!("@");
                } else if self.snake.contains(&(x, y)) {
                    print!("O");
                } else if (x, y) == self.food {
                    print!("X");
                } else {
                    print!(" ");
                }
            }
            println!("|\r");
        }

        println!("{}\r", border);
        let help_text = if matches!(self.mode, ControlMode::Auto { .. }) {
            "auto mode (q to quit)"
        } else {
            "arrow keys to move, q to quit"
        };

        println!("Score: {}   ({})\r", self.score, help_text);

        stdout.flush()?;
        Ok(())
    }

    fn next_head(&self) -> Point {
        (
            self.snake[0].0 + self.direction.0,
            self.snake[0].1 + self.direction.1,
        )
    }

    fn apply_agent_decision(&mut self) {
        if let Some(agent) = &self.agent {
            let state = self.encode_state();
            let action_index = agent.best_action_index(state);
            self.apply_turn_action(ACTIONS[action_index]);
        }
    }

    fn apply_turn_action(&mut self, action: TurnAction) {
        let direction = turn_direction(self.direction, action);
        self.next_direction = direction;
    }

    fn encode_state(&self) -> usize {
        let head = self.snake[0];
        let dir = self.direction;

        // The encoded state is 11 binary features packed into one integer index.
        let bits = [
            self.is_danger_ahead(turn_direction(dir, TurnAction::Straight)),
            self.is_danger_ahead(turn_direction(dir, TurnAction::Left)),
            self.is_danger_ahead(turn_direction(dir, TurnAction::Right)),
            dir == (0, -1),
            dir == (0, 1),
            dir == (-1, 0),
            dir == (1, 0),
            self.food.0 < head.0,
            self.food.0 > head.0,
            self.food.1 < head.1,
            self.food.1 > head.1,
        ];

        bits.iter().enumerate().fold(0usize, |acc, (idx, flag)| {
            if *flag {
                acc | (1 << idx)
            } else {
                acc
            }
        })
    }

    fn is_danger_ahead(&self, direction: Point) -> bool {
        let head = self.snake[0];
        let next = (head.0 + direction.0, head.1 + direction.1);
        self.hit_wall(next) || self.hit_self(next)
    }

    fn reset(&mut self) {
        self.snake = vec![(5, 5)];
        self.direction = (1, 0);
        self.next_direction = (1, 0);
        self.food = spawn_food(&self.snake);
        self.score = 0;
        self.steps_since_food = 0;
    }

    fn train_agent(&mut self, episodes: usize) -> RlAgent {
        let mut agent = RlAgent::new();

        for _ in 0..episodes {
            self.reset();

            let mut done = false;
            let mut steps = 0;

            while !done && steps < 1000 {
                let state = self.encode_state();
                let action_index = agent.select_action_index(state, true);
                self.apply_turn_action(ACTIONS[action_index]);

                let result = self.step();
                let next_state = self.encode_state();
                done = matches!(result.outcome, StepOutcome::Quit);

                agent.update(state, action_index, result.reward, next_state, done);
                steps += 1;
            }

            agent.decay_exploration();
        }

        agent.epsilon = 0.0;
        agent
    }

    fn hit_wall(&self, head: Point) -> bool {
        head.0 < 0 || head.0 >= WIDTH || head.1 < 0 || head.1 >= HEIGHT
    }

    fn hit_self(&self, head: Point) -> bool {
        self.snake.contains(&head)
    }
}

fn spawn_food(snake: &[Point]) -> Point {
    loop {
        let candidate = (
            rand::random::<u8>() as i32 % WIDTH,
            rand::random::<u8>() as i32 % HEIGHT,
        );

        if !snake.contains(&candidate) {
            return candidate;
        }
    }
}

fn turn_direction(direction: Point, action: TurnAction) -> Point {
    match action {
        TurnAction::Straight => direction,
        TurnAction::Left => (-direction.1, direction.0),
        TurnAction::Right => (direction.1, -direction.0),
    }
}

fn manhattan_distance(a: Point, b: Point) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}
