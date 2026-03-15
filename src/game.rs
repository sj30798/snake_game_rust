use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::{cursor, execute};
use std::io::{self, Write};
use std::time::Duration;

const WIDTH: i32 = 20;
const HEIGHT: i32 = 20;
const TICK_MS: u64 = 200;

type Point = (i32, i32);

enum StepOutcome {
    Continue,
    Quit,
}

pub struct Game {
    snake: Vec<Point>,
    direction: Point,
    next_direction: Point,
    food: Point,
    score: u32,
}

impl Game {
    pub fn new() -> Self {
        let snake = vec![(5, 5)];
        let food = spawn_food(&snake);

        Self {
            snake,
            direction: (1, 0),
            next_direction: (1, 0),
            food,
            score: 0,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        let mut stdout = io::stdout();

        loop {
            if self.read_input()? {
                break;
            }

            if let StepOutcome::Quit = self.step() {
                break;
            }

            self.draw(&mut stdout)?;
            std::thread::sleep(Duration::from_millis(TICK_MS));
        }

        println!("\nGame over! Final score: {}", self.score);
        Ok(())
    }

    fn read_input(&mut self) -> io::Result<bool> {
        while event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Up if self.direction != (0, 1) => self.next_direction = (0, -1),
                    KeyCode::Down if self.direction != (0, -1) => self.next_direction = (0, 1),
                    KeyCode::Left if self.direction != (1, 0) => self.next_direction = (-1, 0),
                    KeyCode::Right if self.direction != (-1, 0) => self.next_direction = (1, 0),
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
                    _ => {}
                }
            }
        }

        Ok(false)
    }

    fn step(&mut self) -> StepOutcome {
        self.direction = self.next_direction;
        let new_head = self.next_head();

        if self.hit_wall(new_head) || self.hit_self(new_head) {
            return StepOutcome::Quit;
        }

        self.snake.insert(0, new_head);

        if new_head == self.food {
            self.score += 1;
            self.food = spawn_food(&self.snake);
        } else {
            self.snake.pop();
        }

        StepOutcome::Continue
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
        println!("Score: {}   (arrow keys to move, q to quit)\r", self.score);

        stdout.flush()?;
        Ok(())
    }

    fn next_head(&self) -> Point {
        (
            self.snake[0].0 + self.direction.0,
            self.snake[0].1 + self.direction.1,
        )
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
