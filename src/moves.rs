use super::board::{Board, Coordinate, Square};
use super::hand::Hands;
use super::judge::{Judge, Outcome};
use crate::error::GamePlayError;

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
    pub fn make_move<'a>(
        &'a mut self,
        game_move: Move,
        hands: &'a mut Hands,
        judge: &Judge,
    ) -> Result<(), GamePlayError> {
        match game_move {
            Move::Place {
                player,
                tile,
                position,
            } => {
                if let Square::Occupied(..) = self.get(position)? {
                    return Err(GamePlayError::OccupiedPlace);
                }

                if position != self.get_root(player)?
                    && !self.neighbouring_squares(position).iter().any(
                        |&(_, square)| match square {
                            Square::Occupied(p, _) => p == player,
                            _ => false,
                        },
                    )
                {
                    return Err(GamePlayError::NonAdjacentPlace);
                }

                hands.use_tile(player, tile)?;
                self.set(position, player, tile)?;
                self.resolve_attack(player, position, judge, hands);
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
    fn resolve_attack(
        &mut self,
        player: usize,
        position: Coordinate,
        judge: &Judge,
        hands: &mut Hands,
    ) {
        let (attackers, defenders) = self.collect_combanants(player, position);
        let attacking_words = self
            .word_strings(&attackers)
            .expect("Words were just found and should be valid");
        let defending_words = self
            .word_strings(&defenders)
            .expect("Words were just found and should be valid");
        match judge.battle(attacking_words, defending_words) {
            Outcome::NoBattle => {}
            Outcome::DefenderWins => {
                for word in attackers {
                    for square in word {
                        if let Ok(Square::Occupied(_, letter)) = self.get(square) {
                            hands.return_tile(letter);
                        }
                        self.clear(square);
                    }
                }
            }
            Outcome::AttackerWins(losers) => {
                for defender_index in losers {
                    for square in defenders
                        .get(defender_index)
                        .expect("Losers should only contain valid squares")
                    {
                        if let Ok(Square::Occupied(_, letter)) = self.get(*square) {
                            hands.return_tile(letter);
                        }
                        self.clear(*square);
                    }
                }
            }
        }

        self.truncate(hands);
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
            .filter(|(_, square)| match square {
                Square::Occupied(adjacent_player, _) => player != *adjacent_player,
                _ => false,
            })
            .flat_map(|(position, _)| self.get_words(*position))
            .collect();
        (attackers, defenders)
    }
}

#[cfg(test)]
mod tests {
    use crate::board::{tests as BoardUtils, Direction};

    use super::super::bag::tests as TileUtils;
    use super::*;

    pub fn short_dict() -> Judge {
        Judge::new(vec!["BIG", "FAT", "JOLLY", "AND", "SILLY", "FOLK", "ARTS"]) // TODO: Collins 2018 list
    }

    #[test]
    fn invalid_placement_locations() {
        let mut b = Board::new(3, 1);
        let mut hands = Hands::new(2, 7, TileUtils::trivial_bag());

        let position = Coordinate { x: 10, y: 10 };
        let out_of_bounds = Move::Place {
            player: 0,
            tile: 'A',
            position,
        };
        assert_eq!(
            b.make_move(out_of_bounds, &mut hands, &short_dict()),
            Err(GamePlayError::OutSideBoardDimensions { position })
        );

        let position = Coordinate { x: 10, y: 0 };
        let out_of_bounds = Move::Place {
            player: 0,
            tile: 'A',
            position,
        };
        assert_eq!(
            b.make_move(out_of_bounds, &mut hands, &short_dict()),
            Err(GamePlayError::OutSideBoardDimensions { position })
        );

        let position = Coordinate { x: 0, y: 0 };
        let dead = Move::Place {
            player: 0,
            tile: 'A',
            position,
        };
        assert_eq!(
            b.make_move(dead, &mut hands, &short_dict()),
            Err(GamePlayError::InvalidPosition { position })
        );
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
                &mut hands,
                &short_dict()
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
                &mut hands,
                &short_dict()
            ),
            Err(GamePlayError::OccupiedPlace)
        );
        // Can't place at a diagonal
        assert_eq!(
            b.make_move(
                Move::Place {
                    player: 0,
                    tile: 'A',
                    position: Coordinate { x: 0, y: 1 }
                },
                &mut hands,
                &short_dict()
            ),
            Err(GamePlayError::NonAdjacentPlace)
        );
        // Can place directly above
        assert_eq!(
            b.make_move(
                Move::Place {
                    player: 0,
                    tile: 'A',
                    position: Coordinate { x: 1, y: 1 }
                },
                &mut hands,
                &short_dict()
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
                &mut hands,
                &short_dict()
            ),
            Err(GamePlayError::OccupiedPlace)
        );

        assert_eq!(
            b.make_move(
                Move::Swap {
                    player: 0,
                    positions: [Coordinate { x: 1, y: 1 }, Coordinate { x: 1, y: 0 }]
                },
                &mut hands,
                &short_dict()
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
                &mut hands,
                &short_dict()
            ),
            Err(GamePlayError::NonExistentPlayer { index: 2 })
        );

        assert_eq!(
            b.make_move(
                Move::Place {
                    player: 0,
                    tile: '&',
                    position: Coordinate { x: 1, y: 0 }
                },
                &mut hands,
                &short_dict()
            ),
            Err(GamePlayError::PlayerDoesNotHaveTile {
                player: 0,
                tile: '&'
            })
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
        let left_attacker: Vec<Coordinate> =
            (0..=2).rev().map(|x| Coordinate { x, y: 2 }).collect();
        let cross_defender: Vec<Coordinate> = (1..=3).map(|x| Coordinate { x, y: 3 }).collect();
        let short_cross_defender: Vec<Coordinate> =
            (2..=3).map(|x| Coordinate { x, y: 3 }).collect();

        // There are at most 4 squares contributing combatants.
        // Either 1 attacker with 1, 2, or 3 defenders
        // 2 attackers with 1 or 2 defenders
        // Note, 3 attackers are impossible because the letter being placed will combine two of the words into one

        // 1v1
        let mut one_v_one = BoardUtils::from_string(
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
        let mut one_v_two = BoardUtils::from_string(
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
        let mut one_v_three = BoardUtils::from_string(
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
        one_v_three.set(middle, 0, 'A').unwrap();

        assert_eq!(
            one_v_three.collect_combanants(0, middle),
            (
                vec![middle_attacker.clone()],
                vec![
                    middle_defender.clone(),
                    cross_defender.clone(),
                    right_defender.clone(),
                    left_defender.clone(),
                ]
            )
        );

        // 2v2
        let mut two_v_two = BoardUtils::from_string(
            [
                "X X M _ _",
                "X _ D _ _",
                "L F _ R _",
                "_ _ M T _",
                "_ _ D D _",
            ]
            .join("\n"),
            vec![Coordinate { x: 2, y: 0 }, Coordinate { x: 2, y: 4 }],
            vec![Direction::North, Direction::South],
        )
        .unwrap();
        two_v_two.set(middle, 0, 'A').unwrap();
        assert_eq!(
            two_v_two.collect_combanants(0, middle),
            (
                vec![middle_attacker, left_attacker],
                vec![middle_defender, short_cross_defender, right_defender],
            )
        );
    }

    #[test]
    fn resolve_successful_attack() {
        let mut b = BoardUtils::from_string(
            [
                "_ S X _ _",
                "_ T _ _ _",
                "_ R _ _ _",
                "_ _ I _ _",
                "_ _ T _ _",
            ]
            .join("\n"),
            vec![Coordinate { x: 2, y: 0 }, Coordinate { x: 2, y: 4 }],
            vec![Direction::North, Direction::South],
        )
        .unwrap();
        let mut hands = Hands::new(2, 7, TileUtils::trivial_bag());

        b.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 1, y: 3 },
            },
            &mut hands,
            &short_dict(),
        )
        .unwrap();

        assert_eq!(
            b.to_string(),
            [
                "_ S X _ _",
                "_ T _ _ _",
                "_ R _ _ _",
                "_ A _ _ _",
                "_ _ _ _ _",
            ]
            .join("\n"),
        )
    }

    #[test]
    fn resolve_failed_attack() {
        let mut b = BoardUtils::from_string(
            [
                "_ X X _ _",
                "_ T _ _ _",
                "_ R _ _ _",
                "_ _ I _ _",
                "_ _ T _ _",
            ]
            .join("\n"),
            vec![Coordinate { x: 2, y: 0 }, Coordinate { x: 2, y: 4 }],
            vec![Direction::North, Direction::South],
        )
        .unwrap();
        let mut hands = Hands::new(2, 7, TileUtils::trivial_bag());

        b.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 1, y: 3 },
            },
            &mut hands,
            &short_dict(),
        )
        .unwrap();

        assert_eq!(
            b.to_string(),
            [
                "_ _ X _ _",
                "_ _ _ _ _",
                "_ _ _ _ _",
                "_ _ I _ _",
                "_ _ T _ _",
            ]
            .join("\n"),
        )
    }

    #[test]
    fn resolve_truncation() {
        let mut b = BoardUtils::from_string(
            [
                "_ S X _ _",
                "_ T _ _ _",
                "_ R _ X _",
                "_ _ B X _",
                "_ _ I _ _",
                "_ _ G _ _",
            ]
            .join("\n"),
            vec![Coordinate { x: 2, y: 0 }, Coordinate { x: 2, y: 5 }],
            vec![Direction::North, Direction::South],
        )
        .unwrap();
        let mut hands = Hands::new(2, 7, TileUtils::trivial_bag());
        let mut test_hands = Hands::new(2, 7, TileUtils::trivial_bag());
        assert_eq!(hands, test_hands);

        b.make_move(
            Move::Place {
                player: 0,
                tile: 'A',
                position: Coordinate { x: 1, y: 3 },
            },
            &mut hands,
            &short_dict(),
        )
        .unwrap();

        for letter in ['B', 'X', 'X'] {
            test_hands.return_tile(letter);
        }
        assert_eq!(hands, test_hands);

        assert_eq!(
            b.to_string(),
            [
                "_ S X _ _",
                "_ T _ _ _",
                "_ R _ _ _",
                "_ A _ _ _",
                "_ _ I _ _",
                "_ _ G _ _",
            ]
            .join("\n"),
        );
    }
}
