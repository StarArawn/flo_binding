use crate::*;

use flo_rope::*;

use futures::executor;
use futures::prelude::*;

#[test]
fn mutable_rope_sends_changes_to_stream() {
    // Create a rope that copies changes from a mutable rope
    let mutable_rope        = RopeBindingMut::<usize, ()>::new();
    let mut rope_stream     = mutable_rope.follow_changes();

    // Write some data to the mutable rope
    mutable_rope.replace(0..0, vec![1, 2, 3, 4]);

    // Should get sent to the stream
    executor::block_on(async move {
        let next = rope_stream.next().await;

        assert!(next == Some(RopeAction::Replace(0..0, vec![1,2,3,4])));
    });
}

#[test]
fn pull_from_mutable_binding() {
    // Create a rope that copies changes from a mutable rope
    let mutable_rope        = RopeBindingMut::<usize, ()>::new();
    let rope_copy           = RopeBinding::from_stream(mutable_rope.follow_changes());
    let mut rope_stream     = rope_copy.follow_changes();

    // Write some data to the mutable rope
    mutable_rope.replace(0..0, vec![1, 2, 3, 4]);

    // Wait for the change to arrive at the copy
    executor::block_on(async move {
        let next = rope_stream.next().await;
        assert!(next == Some(RopeAction::Replace(0..0, vec![1,2,3,4])))
    });

    // Read from the copy
    assert!(rope_copy.len() == 4);
    assert!(rope_copy.read_cells(0..4).collect::<Vec<_>>() == vec![1, 2, 3, 4]);
}

#[test]
fn concatenate_ropes() {
    // Create a LHS and RHS rope and a concatenation of both
    let lhs                 = RopeBindingMut::<usize, ()>::new();
    let rhs                 = RopeBindingMut::<usize, ()>::new();
    let chain               = lhs.chain(&rhs);

    // We need to wait for the changes to arrive on the concatenated rope to avoid racing when reading back
    let mut follow_chain    = chain.follow_changes();

    // Add to LHS
    lhs.replace(0..0, vec![1, 2, 3]);
    executor::block_on(async { follow_chain.next().await });
    println!("{:?}", chain.read_cells(0..3).collect::<Vec<_>>());
    assert!(chain.read_cells(0..3).collect::<Vec<_>>() == vec![1, 2, 3]);

    // Add to RHS
    rhs.replace(0..0, vec![10, 11, 12]);
    executor::block_on(async { follow_chain.next().await });
    println!("{:?}", chain.read_cells(0..6).collect::<Vec<_>>());
    assert!(chain.read_cells(0..6).collect::<Vec<_>>() == vec![1, 2, 3, 10, 11, 12]);

    // Edit LHS
    lhs.replace(1..2, vec![4, 5, 6]);
    executor::block_on(async { follow_chain.next().await });
    println!("{:?}", chain.read_cells(0..8).collect::<Vec<_>>());
    assert!(chain.read_cells(0..8).collect::<Vec<_>>() == vec![1, 4, 5, 6, 3, 10, 11, 12]);

    // Edit RHS
    rhs.replace(1..2, vec![20, 21, 22]);
    executor::block_on(async { follow_chain.next().await });
    println!("{:?}", chain.read_cells(0..10).collect::<Vec<_>>());
    assert!(chain.read_cells(0..10).collect::<Vec<_>>() == vec![1, 4, 5, 6, 3, 10, 20, 21, 22, 12]);
}

#[test]
fn map_ropes() {
    // Create a rope with some numbers in it
    let rope            = RopeBindingMut::<usize, ()>::new();
    rope.replace(0..0, vec![1, 2, 3]);

    // Create a mapped rope that adds one to the numbers
    let add_one         = rope.map(|val| val+1);

    // Check that it changes as the numbers change
    let mut follow_add  = add_one.follow_changes();

    executor::block_on(async { follow_add.next().await });
    assert!(add_one.read_cells(0..3).collect::<Vec<_>>() == vec![2, 3, 4]);

    rope.replace(1..1, vec![8, 9, 10]);
    executor::block_on(async { follow_add.next().await });
    assert!(add_one.read_cells(0..6).collect::<Vec<_>>() == vec![2, 9, 10, 11, 3, 4]);
}

#[test]
fn computed_rope() {
    // Create a length binding and compute a rope from it
    let length          = bind(0);
    let length_copy     = length.clone();
    let rope            = RopeBinding::<_, ()>::computed(move || (0..length_copy.get()).into_iter().map(|idx| idx));

    // Follow a the rope changes so we can sync up with the changes
    let mut follow_rope = rope.follow_changes();

    // Increase length to 1
    length.set(1);
    executor::block_on(async { follow_rope.next().await });
    assert!(rope.len() == 1);
    assert!(rope.read_cells(0..1).collect::<Vec<_>>() == vec![0]);

    length.set(3);
    executor::block_on(async { follow_rope.next().await });
    assert!(rope.len() == 3);
    assert!(rope.read_cells(0..3).collect::<Vec<_>>() == vec![0, 1, 2]);

    length.set(2);
    executor::block_on(async { follow_rope.next().await });
    assert!(rope.len() == 2);
    assert!(rope.read_cells(0..2).collect::<Vec<_>>() == vec![0, 1]);

    length.set(10);
    executor::block_on(async { follow_rope.next().await });
    assert!(rope.len() == 10);
    assert!(rope.read_cells(0..10).collect::<Vec<_>>() == vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn computed_rope_using_diffs_1() {
    // Create a length binding and compute a rope from it
    let length          = bind(0);
    let length_copy     = length.clone();
    let rope            = RopeBinding::<_, ()>::computed_difference(move || (0..length_copy.get()).into_iter().map(|idx| idx));

    // Follow a the rope changes so we can sync up with the changes
    let mut follow_rope = rope.follow_changes();

    // Increase length to 1
    length.set(1);
    executor::block_on(async { follow_rope.next().await });
    assert!(rope.len() == 1);
    assert!(rope.read_cells(0..1).collect::<Vec<_>>() == vec![0]);

    length.set(3);
    executor::block_on(async { follow_rope.next().await });
    assert!(rope.len() == 3);
    assert!(rope.read_cells(0..3).collect::<Vec<_>>() == vec![0, 1, 2]);

    length.set(2);
    executor::block_on(async { follow_rope.next().await });
    assert!(rope.len() == 2);
    assert!(rope.read_cells(0..2).collect::<Vec<_>>() == vec![0, 1]);

    length.set(10);
    executor::block_on(async { follow_rope.next().await });
    assert!(rope.len() == 10);
    assert!(rope.read_cells(0..10).collect::<Vec<_>>() == vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn computed_rope_using_diffs_2() {
    // Create a length binding and compute a rope from it
    let length          = bind(0);
    let length_copy     = length.clone();
    let rope            = RopeBinding::<_, ()>::computed_difference(move || (0..length_copy.get()).into_iter().map(|idx| idx));

    // Follow a the rope changes so we can sync up with the changes
    let mut follow_rope = rope.follow_changes();

    // Increase length to 1
    length.set(1);
    let diff = executor::block_on(async { follow_rope.next().await });
    assert!(diff == Some(RopeAction::Replace(0..0, vec![0])));

    length.set(3);
    let diff = executor::block_on(async { follow_rope.next().await });
    assert!(diff == Some(RopeAction::Replace(1..1, vec![1, 2])));

    length.set(2);
    let diff = executor::block_on(async { follow_rope.next().await });
    assert!(diff == Some(RopeAction::Replace(2..3, vec![])));

    length.set(10);
    let diff = executor::block_on(async { follow_rope.next().await });
    assert!(diff == Some(RopeAction::Replace(2..2, vec![2, 3, 4, 5, 6, 7, 8, 9])));
}
