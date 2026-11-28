//! Поиск пути по сетке: A* с бюджетом раскрытий и жадный шаг как запасной вариант.
//!
//! Существа не пересчитывают путь каждый тик: путь кэшируется в
//! [`crate::entity::Entity::path`], а [`PathFinder`] переиспользует буферы,
//! поэтому поиск не выделяет память на каждом вызове.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::grid::{TilePos, NEIGHBORS8};

/// Переиспользуемый A* по сетке.
#[derive(Clone, Debug)]
pub struct PathFinder {
    width: i32,
    height: i32,
    g_score: Vec<u32>,
    came_from: Vec<i32>,
    visited_at: Vec<u32>,
    stamp: u32,
}

impl PathFinder {
    /// Создаёт искатель для сетки заданного размера.
    pub fn new(width: u32, height: u32) -> Self {
        let width = width.max(1) as i32;
        let height = height.max(1) as i32;
        let cells = (width as usize) * (height as usize);
        Self {
            width,
            height,
            g_score: vec![0; cells],
            came_from: vec![-1; cells],
            visited_at: vec![0; cells],
            stamp: 0,
        }
    }

    /// Размер сетки, под который настроен искатель.
    pub fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    /// Ищет путь от `start` до `goal`.
    ///
    /// `walkable` должен возвращать `true` для проходимых клеток. Цель
    /// считается достижимой даже если она непроходима (например, дом):
    /// существу достаточно подойти к ней вплотную.
    ///
    /// Возвращает список клеток **без** стартовой, или `None`, если путь не
    /// найден либо превышен `budget` раскрытий.
    pub fn find(
        &mut self,
        start: TilePos,
        goal: TilePos,
        walkable: impl Fn(TilePos) -> bool,
        budget: u32,
    ) -> Option<Vec<TilePos>> {
        if start == goal {
            return Some(Vec::new());
        }
        if !self.in_bounds(start) || !self.in_bounds(goal) {
            return None;
        }

        self.begin_search();
        let start_index = self.index(start);
        let goal_index = self.index(goal);
        self.visit(start_index);
        self.g_score[start_index] = 0;
        self.came_from[start_index] = -1;

        let mut open = BinaryHeap::new();
        open.push(Reverse((heuristic(start, goal), 0u32, start_index as u32)));

        let mut expanded = 0u32;
        while let Some(Reverse((_estimate, cost, current))) = open.pop() {
            let current = current as usize;
            if current == goal_index {
                return Some(self.reconstruct(goal_index, start_index));
            }
            if cost > self.g_score[current] {
                continue; // устаревшая запись в очереди
            }
            expanded += 1;
            if expanded > budget {
                return None;
            }

            let pos = self.pos_of(current);
            for (dx, dy) in NEIGHBORS8 {
                let next = pos.offset(dx, dy);
                if !self.in_bounds(next) {
                    continue;
                }
                let next_index = self.index(next);
                if next_index != goal_index && !walkable(next) {
                    continue;
                }
                let step_cost = if dx != 0 && dy != 0 { 14 } else { 10 };
                let next_cost = cost + step_cost;
                if self.is_visited(next_index) && next_cost >= self.g_score[next_index] {
                    continue;
                }
                self.visit(next_index);
                self.g_score[next_index] = next_cost;
                self.came_from[next_index] = current as i32;
                open.push(Reverse((
                    next_cost + heuristic(next, goal),
                    next_cost,
                    next_index as u32,
                )));
            }
        }

        None
    }

    fn begin_search(&mut self) {
        // Метки «посещено» хранятся номером поиска, поэтому очистка не нужна.
        if self.stamp == u32::MAX {
            self.visited_at.iter_mut().for_each(|mark| *mark = 0);
            self.stamp = 0;
        }
        self.stamp += 1;
    }

    fn visit(&mut self, index: usize) {
        let stamp = self.stamp;
        self.visited_at[index] = stamp;
    }

    fn is_visited(&self, index: usize) -> bool {
        self.visited_at[index] == self.stamp
    }

    fn in_bounds(&self, p: TilePos) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < self.width && p.y < self.height
    }

    fn index(&self, p: TilePos) -> usize {
        (p.y as usize) * (self.width as usize) + (p.x as usize)
    }

    fn pos_of(&self, index: usize) -> TilePos {
        let width = self.width as usize;
        TilePos::new((index % width) as i32, (index / width) as i32)
    }

    fn reconstruct(&self, goal_index: usize, start_index: usize) -> Vec<TilePos> {
        let mut path = Vec::new();
        let mut current = goal_index as i32;
        let mut guard = 0usize;

        while current >= 0 && current as usize != start_index {
            path.push(self.pos_of(current as usize));
            current = self.came_from[current as usize];
            guard += 1;
            if guard > self.g_score.len() {
                break;
            }
        }

        path.reverse();
        path
    }
}

/// Один жадный шаг в сторону цели (обходит препятствия «на месте»).
///
/// Используется, когда полноценный A* не уложился в бюджет: так существо
/// всё равно продолжает двигаться, а не замирает.
pub fn greedy_step(
    from: TilePos,
    to: TilePos,
    walkable: impl Fn(TilePos) -> bool,
) -> Option<TilePos> {
    if from == to {
        return None;
    }
    let dx = (to.x - from.x).signum();
    let dy = (to.y - from.y).signum();

    let candidates = [
        from.offset(dx, dy),
        from.offset(dx, 0),
        from.offset(0, dy),
        from.offset(dy, dx),
        from.offset(-dy, -dx),
        from.offset(dx, -dy),
        from.offset(-dx, dy),
    ];

    candidates
        .into_iter()
        .find(|candidate| *candidate != from && walkable(*candidate))
}

fn heuristic(a: TilePos, b: TilePos) -> u32 {
    let dx = (a.x - b.x).unsigned_abs();
    let dy = (a.y - b.y).unsigned_abs();
    let (min, max) = if dx < dy { (dx, dy) } else { (dy, dx) };
    min * 14 + (max - min) * 10
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::pos;
    use std::collections::HashSet;

    fn open(_: TilePos) -> bool {
        true
    }

    #[test]
    fn straight_line_path() {
        let mut finder = PathFinder::new(32, 32);
        let path = finder.find(pos(1, 1), pos(6, 1), open, 2000).expect("путь");
        assert_eq!(path.last().copied(), Some(pos(6, 1)));
        assert_eq!(path.len(), 5);
        assert_eq!(path.first().copied(), Some(pos(2, 1)));
    }

    #[test]
    fn path_avoids_walls() {
        let mut finder = PathFinder::new(40, 40);
        let mut blocked = HashSet::new();
        for y in 0..30 {
            blocked.insert((20, y));
        }
        let walkable = |p: TilePos| !blocked.contains(&(p.x, p.y));
        let path = finder
            .find(pos(5, 5), pos(35, 5), walkable, 20_000)
            .expect("путь в обход стены");
        assert!(path.contains(&pos(20, 30)) || path.iter().any(|p| p.y >= 30));
        assert!(path.iter().all(|p| !blocked.contains(&(p.x, p.y))));
        assert_eq!(path.last().copied(), Some(pos(35, 5)));
    }

    #[test]
    fn unreachable_goal_returns_none() {
        let mut finder = PathFinder::new(20, 20);
        let wall = |p: TilePos| p.x != 10;
        assert_eq!(finder.find(pos(2, 2), pos(18, 18), wall, 5000), None);
    }

    #[test]
    fn budget_is_respected() {
        let mut finder = PathFinder::new(64, 64);
        assert_eq!(finder.find(pos(0, 0), pos(63, 63), open, 1), None);
    }

    #[test]
    fn same_tile_is_empty_path() {
        let mut finder = PathFinder::new(8, 8);
        let path = finder.find(pos(3, 3), pos(3, 3), open, 10).expect("путь");
        assert!(path.is_empty());
    }

    #[test]
    fn out_of_bounds_is_none() {
        let mut finder = PathFinder::new(8, 8);
        assert_eq!(finder.find(pos(3, 3), pos(100, 100), open, 100), None);
        assert_eq!(finder.find(pos(-1, 3), pos(3, 3), open, 100), None);
    }

    #[test]
    fn greedy_step_moves_towards_goal() {
        let step = greedy_step(pos(5, 5), pos(9, 5), open).expect("шаг");
        assert_eq!(step, pos(6, 5));
        let step = greedy_step(pos(5, 5), pos(9, 9), open).expect("шаг");
        assert_eq!(step, pos(6, 6));
        assert_eq!(greedy_step(pos(1, 1), pos(1, 1), open), None);
    }

    #[test]
    fn greedy_step_avoids_blocked_tile() {
        let blocked = |p: TilePos| p != pos(6, 6);
        let step = greedy_step(pos(5, 5), pos(9, 9), blocked).expect("шаг");
        assert_ne!(step, pos(6, 6));
    }
}
