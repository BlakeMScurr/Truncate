use super::board::{Board, Coordinate, Square};
use super::hand::Hands;

pub enum Move {
    // TODO: make Move a struct and make player a top level property of it
    Place {
        player: usize,
        tile: char,
        position: Coordinate,
    },
    Swap {
        player: usize,
        positions: [Coordinate; 2],
    },
}

// TODO: is it weird to implement this on Board here rather than on Move?
impl Board {
    pub fn make_move<'a>(&'a mut self, game_move: Move, hands: &'a mut Hands) -> Result<(), &str> {
        match game_move {
            Move::Place {
                player,
                tile,
                position,
            } => {
                match self.get(position) {
                    Err(_) => return Err("Couldn't get square"), // TODO: propogate the internal error, ideally succinctly with the ? operator. This is hard because of a borrow checker issue https://github.com/rust-lang/rfcs/blob/master/text/2094-nll.md#problem-case-3-conditional-control-flow-across-functions
                    Ok(sq) => match sq {
                        Square::Occupied(_, _) => {
                            return Err("Cannot place a tile in an occupied square")
                        }
                        Square::Empty => {}
                    },
                };

                let root = match self.get_root(player) {
                    Err(_) => return Err("Invalid player"), // TODO: propogate using ? with Polonius https://github.com/rust-lang/rfcs/blob/master/text/2094-nll.md#problem-case-3-conditional-control-flow-across-functions
                    Ok(coordinate) => coordinate,
                };

                if position != root
                    && self
                        .neighbouring_squares(position)
                        .iter()
                        .filter(|square| match (*square).1 {
                            Square::Empty => false,
                            Square::Occupied(p, _) => p == player,
                        })
                        .count()
                        == 0
                {
                    return Err("Must place tile on square that neighbours one of your already placed tiles, or on your root");
                }

                hands.use_tile(player, tile)?; // Use tile checks that the player is valid and has that letter
                self.set(position, player, tile)?;

                Ok(())
            }
            Move::Swap { player, positions } => self.swap(player, positions),
        }
    }

    // If any attacking word is invalid, or all defending words are valid and stronger than the longest attacking words
    //   - All attacking words die
    //   - Attacking tiles are truncated
    // Otherwise
    //   - Weak and invalid defending words die
    //   - Any remaining defending letters adjacent to the attacking tile die
    //   - Defending tiles are truncated
    fn resolve_attack(&self, player: usize, position: Coordinate) {
        let (attackers, defenders) = self.collect_combanants(player, position);
    }

    fn collect_combanants(
        &self,
        player: usize,
        position: Coordinate,
    ) -> (Vec<Vec<Coordinate>>, Vec<Vec<Coordinate>>) {
        let attackers = self.get_words(position);
        // Any neighbouring square belonging to another player is attacked. The words containing those squares are the defenders.
        let defenders = self
            .neighbouring_squares(position)
            .iter()
            .filter(|pos| {
                if let Square::Occupied(adjacent_player, _) = pos.1 {
                    player != adjacent_player
                } else {
                    false
                }
            })
            .flat_map(|(position, _)| self.get_words(*position))
            .collect();
        (attackers, defenders)
    }
}

#[cfg(test)]
mod tests {
    use crate::board::Direction;

    use super::super::bag::tests as TileUtils;
    use super::*;

    #[test]
    fn invalid_placement_locations() {
        let mut b = Board::new(3, 1);
        let mut hands = Hands::new(2, 7, TileUtils::trivial_bag());

        let out_of_bounds = Move::Place {
            player: 0,
            tile: 'A',
            position: Coordinate { x: 10, y: 10 },
        };
        assert_eq!(
            b.make_move(out_of_bounds, &mut hands),
            // Err("y-coordinate is too large for board height") // <- TODO
            Err("Couldn't get square")
        );

        let out_of_bounds = Move::Place {
            player: 0,
            tile: 'A',
            position: Coordinate { x: 10, y: 0 },
        };
        assert_eq!(
            b.make_move(out_of_bounds, &mut hands),
            // Err("x-coordinate is too large for board width") // <- TODO
            Err("Couldn't get square")
        );

        let dead = Move::Place {
            player: 0,
            tile: 'A',
            position: Coordinate { x: 0, y: 0 },
        };
        assert_eq!(b.make_move(dead, &mut hands), Err("Couldn't get square"));
    }

    #[test]
    fn can_place_and_swap() {
        let mut b = Board::new(3, 1);
        let mut hands = Hands::new(1, 7, TileUtils::a_b_bag());

        // Places on the root
        assert_eq!(
            b.make_move(
                Move::Place {
                    player: 0,
                    tile: 'A',
                    position: Coordinate { x: 1, y: 0 }
                },
                &mut hands
            ),
            Ok(())
        );
        // Can't place on the same place again
        assert_eq!(
            b.make_move(
                Move::Place {
                    player: 0,
                    tile: 'A',
                    position: Coordinate { x: 1, y: 0 }
                },
                &mut hands
            ),
            Err("Cannot place a tile in an occupied square")
        );
        // Can't place at a diagonal
        assert_eq!(
            b.make_move(Move::Place{player: 0, tile: 'A', position: Coordinate { x: 0, y: 1 }}, &mut hands),
            Err("Must place tile on square that neighbours one of your already placed tiles, or on your root")
        );
        // Can place directly above
        assert_eq!(
            b.make_move(
                Move::Place {
                    player: 0,
                    tile: 'A',
                    position: Coordinate { x: 1, y: 1 }
                },
                &mut hands
            ),
            Ok(())
        );
        // Can't place on the same place again
        assert_eq!(
            b.make_move(
                Move::Place {
                    player: 0,
                    tile: 'A',
                    position: Coordinate { x: 1, y: 1 }
                },
                &mut hands
            ),
            Err("Cannot place a tile in an occupied square")
        );

        assert_eq!(
            b.make_move(
                Move::Swap {
                    player: 0,
                    positions: [Coordinate { x: 1, y: 1 }, Coordinate { x: 1, y: 0 }]
                },
                &mut hands
            ),
            Ok(())
        );
    }

    #[test]
    fn invalid_player_or_tile() {
        let mut b = Board::new(3, 1);
        let mut hands = Hands::default();

        assert_eq!(
            b.make_move(
                Move::Place {
                    player: 2,
                    tile: 'A',
                    position: Coordinate { x: 1, y: 0 }
                },
                &mut hands
            ),
            Err("Invalid player")
        );

        assert_eq!(
            b.make_move(
                Move::Place {
                    player: 0,
                    tile: '&',
                    position: Coordinate { x: 1, y: 0 }
                },
                &mut hands
            ),
            Err("Player doesn't have that tile")
        );
    }

    #[test]
    fn collect_combanants() {
        let middle = Coordinate { x: 2, y: 2 };

        let left_defender: Vec<Coordinate> = (2..=4).map(|y| Coordinate { x: 1, y }).collect();
        let right_defender: Vec<Coordinate> = (2..=4).map(|y| Coordinate { x: 3, y }).collect();
        let middle_defender: Vec<Coordinate> = (3..=4).map(|y| Coordinate { x: 2, y }).collect();
        let middle_attacker: Vec<Coordinate> =
            (0..=2).rev().map(|y| Coordinate { x: 2, y }).collect();

        // There are at most 4 squares contributing combatants.
        // Either 1 attacker with 1, 2, or 3 defenders
        // 2 attackers with 1 or 2 defenders
        // 3 attackers with 1 defender

        // 1v1
        let mut one_v_one = Board::from_string(
            [
                "_ _ M _ _",
                "_ _ D _ _",
                "_ _ _ _ _",
                "_ _ M _ _",
                "_ _ D _ _",
            ]
            .join("\n"),
            vec![Coordinate { x: 2, y: 0 }, Coordinate { x: 2, y: 4 }],
            vec![Direction::North, Direction::South],
        )
        .unwrap();
        one_v_one.set(middle, 0, 'A').unwrap();

        assert_eq!(
            one_v_one.collect_combanants(0, middle),
            (vec![middle_attacker.clone()], vec![middle_defender.clone()])
        );

        // 1v2
        let mut one_v_two = Board::from_string(
            [
                "_ _ M _ _",
                "_ _ D _ _",
                "_ L _ R _",
                "_ F _ T _",
                "_ D R D _",
            ]
            .join("\n"),
            vec![Coordinate { x: 2, y: 0 }, Coordinate { x: 2, y: 4 }],
            vec![Direction::North, Direction::South],
        )
        .unwrap();
        one_v_two.set(middle, 0, 'A').unwrap();

        assert_eq!(
            one_v_two.collect_combanants(0, middle),
            (
                vec![middle_attacker.clone()],
                vec![right_defender.clone(), left_defender.clone()],
            )
        );

        // 1v3
        let mut one_v_three = Board::from_string(
            [
                "_ _ M _ _",
                "_ _ D _ _",
                "_ L _ R _",
                "_ F M T _",
                "_ D D D _",
            ]
            .join("\n"),
            vec![Coordinate { x: 2, y: 0 }, Coordinate { x: 2, y: 4 }],
            vec![Direction::North, Direction::South],
        )
        .unwrap();
        one_v_two.set(middle, 0, 'A').unwrap();
        let cross_defender: Vec<Coordinate> = (1..=3).map(|x| Coordinate { x, y: 3 }).collect();

        // assert_eq!(
        //     one_v_two.collect_combanants(0, middle),
        //     (
        //         vec![middle_attacker],
        //         vec![
        //             left_defender,
        //             middle_defender,
        //             right_defender,
        //             cross_defender
        //         ]
        //     )
        // );
    }
}
