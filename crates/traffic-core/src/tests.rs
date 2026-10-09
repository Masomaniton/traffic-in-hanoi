use super::*;
#[test]
fn configurable_cards_initialize() {
    let game = Game::new(Layout::standard(3).unwrap());
    assert_eq!(game.board.cards().count(), 6);
    let top = Card::from_display(Owner::Heart, 1, 3).unwrap();
    assert_eq!(game.board.square_of(top).unwrap(), Square::new(0, 0));
}
#[test]
fn root_undo_restores_turn_start() {
    let mut game = Game::new(Layout::standard(3).unwrap());
    let card = Card::from_display(Owner::Heart, 1, 3).unwrap();
    assert!(matches!(
        game.apply(Operation::Root(card)).unwrap(),
        ApplyResult::Applied
    ));
    assert!(matches!(game.undo().unwrap(), ApplyResult::Applied));
    assert!(game.history().is_empty());
}

#[test]
fn buried_root_becomes_a_blocked_batch_and_undo_restores_everything() {
    let mut game = Game::new(Layout::standard(3).unwrap());
    let buried = Card::from_display(Owner::Heart, 2, 3).unwrap();
    let top = Card::from_display(Owner::Heart, 1, 3).unwrap();

    assert!(matches!(
        game.apply(Operation::Root(buried)).unwrap(),
        ApplyResult::Blocked(violations) if violations == vec![Violation::ActiveEviction(top)]
    ));
    assert_eq!(
        game.board.card_state(buried).unwrap().status,
        CardStatus::Evicted
    );
    assert_eq!(
        game.board.card_state(top).unwrap().status,
        CardStatus::Evicted
    );

    assert!(matches!(game.undo().unwrap(), ApplyResult::Applied));
    assert_eq!(
        game.board.card_state(buried).unwrap().status,
        CardStatus::Idle
    );
    assert_eq!(game.board.card_state(top).unwrap().status, CardStatus::Idle);
    assert!(game.history().is_empty());
}

#[test]
fn unknown_target_square_blocks_an_intention() {
    let mut game = Game::new(Layout::standard(2).unwrap());
    let card = Card::from_display(Owner::Heart, 1, 2).unwrap();
    game.apply(Operation::Root(card)).unwrap();

    assert!(matches!(
        game.apply(Operation::Intention {
            card,
            target: Support::Base(Square::new(9, 9)),
        })
        .unwrap(),
        ApplyResult::Blocked(violations) if violations.contains(&Violation::UnknownTargetSquare(Square::new(9, 9)))
    ));
}

#[test]
fn premature_end_is_a_reversible_blocked_batch() {
    let mut game = Game::new(Layout::standard(2).unwrap());
    let card = Card::from_display(Owner::Heart, 1, 2).unwrap();
    game.apply(Operation::Root(card)).unwrap();

    assert!(matches!(
        game.apply(Operation::End).unwrap(),
        ApplyResult::Blocked(violations) if violations == vec![Violation::PendingCards]
    ));
    assert!(matches!(game.undo().unwrap(), ApplyResult::Applied));
    assert_eq!(game.history(), vec![HistoryEntry::Root(card)]);
}

#[test]
fn events_replay_to_the_same_state_including_undo() {
    let created = GameCreated::new(Layout::standard(2).unwrap());
    let card = Card::from_display(Owner::Heart, 1, 2).unwrap();
    let mut original = Game::from_created(created.clone());
    original
        .apply_delta(GameDelta::Operation(Operation::Root(card)))
        .unwrap();
    original.apply_delta(GameDelta::Undo).unwrap();

    let events = original.events().to_vec();
    let mut replay = Game::from_created(created);
    for event in events {
        replay.apply_event(event).unwrap();
    }

    assert_eq!(original.sequence(), replay.sequence());
    assert_eq!(original.board, replay.board);
    assert_eq!(original.active_player(), replay.active_player());
    assert_eq!(original.active_card(), replay.active_card());
    assert_eq!(original.history(), replay.history());
}

#[test]
fn fulfilment_history_records_the_card_and_source_support() {
    let mut game = Game::new(Layout::standard(2).unwrap());
    let card = Card::from_display(Owner::Heart, 1, 2).unwrap();
    let lower = Card::from_display(Owner::Heart, 2, 2).unwrap();
    let target = Support::Base(Square::new(0, 1));

    game.apply(Operation::Root(card)).unwrap();
    game.apply(Operation::Intention { card, target }).unwrap();
    game.apply(Operation::Fulfilment(card)).unwrap();

    assert!(game.history().contains(&HistoryEntry::Fulfilment {
        source: Support::Card(lower),
        card,
    }));
}

#[test]
fn cursor_steps_a_complete_history_batch_without_replay() {
    let mut game = Game::new(Layout::standard(2).unwrap());
    let card = Card::from_display(Owner::Heart, 1, 2).unwrap();
    game.apply(Operation::Root(card)).unwrap();
    let mut cursor = HistoryCursor::from_game(game);

    assert!(cursor.step_back_batch());
    assert_eq!(
        cursor.game().board().card_state(card).unwrap().status,
        CardStatus::Idle
    );
    assert!(cursor.step_forward_batch());
    assert_eq!(
        cursor.game().board().card_state(card).unwrap().status,
        CardStatus::Evicted
    );
}

#[test]
fn cursor_ingests_undo_while_reviewing_without_losing_its_position() {
    let created = GameCreated::new(Layout::standard(2).unwrap());
    let card = Card::from_display(Owner::Heart, 1, 2).unwrap();
    let mut game = Game::from_created(created);
    game.apply_delta(GameDelta::Operation(Operation::Root(card)))
        .unwrap();
    let (_, undo_outcome) = game.apply_delta(GameDelta::Undo).unwrap();
    assert!(matches!(undo_outcome, ApplyResult::Applied));

    let mut reviewed = Game::from_created(game.created().clone());
    reviewed.apply_event(game.events()[0].clone()).unwrap();
    let mut cursor = HistoryCursor::from_game(reviewed);
    assert!(cursor.step_back_batch());

    assert!(cursor.apply_event(game.events()[1].clone()));
    assert!(cursor.is_live());
    assert_eq!(
        cursor.game().board().card_state(card).unwrap().status,
        CardStatus::Idle
    );
}

#[test]
fn cursor_rejects_a_gap_transactionally() {
    let mut cursor = HistoryCursor::from_game(Game::new(Layout::standard(2).unwrap()));
    let card = Card::from_display(Owner::Heart, 1, 2).unwrap();
    assert!(!cursor.apply_event(GameEvent::new(
        GameSequence(2),
        GameDelta::Operation(Operation::Root(card)),
    )));
    assert!(cursor.game().history().is_empty());
    assert!(cursor.is_live());
}

#[test]
fn cursor_stays_live_when_an_event_arrives_live() {
    let mut source = Game::new(Layout::standard(2).unwrap());
    let card = Card::from_display(Owner::Heart, 1, 2).unwrap();
    let (event, _) = source
        .apply_delta(GameDelta::Operation(Operation::Root(card)))
        .unwrap();
    let mut cursor = HistoryCursor::from_game(Game::new(Layout::standard(2).unwrap()));
    assert!(cursor.apply_event(event));
    assert!(cursor.is_live());
    assert_eq!(
        cursor.game().board().card_state(card).unwrap().status,
        CardStatus::Evicted
    );
}
