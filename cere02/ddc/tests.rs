use crate::ddc::Error::*;
use ink_env::{
    call, test,
    test::{default_accounts, recorded_events},
    AccountId, DefaultEnvironment,
};
use ink_lang as ink;

/// Imports all the definitions from the outer scope so we can use them here.
use super::*;

type Event = <Ddc as ::ink_lang::BaseEvent>::Type;

fn make_contract() -> Ddc {
    let mut contract = Ddc::new();

    contract.add_tier(2, 2000, 2000, 2000).unwrap();
    contract.add_tier(4, 4000, 4000, 4000).unwrap();
    contract.add_tier(8, 8000, 8000, 8000).unwrap();

    contract
}

/// We test if the default constructor does its job.
#[ink::test]
fn new_works() {
    let contract = make_contract();
    assert_eq!(contract.tier_deposit(1), 2);
    assert_eq!(contract.tier_deposit(2), 4);
    assert_eq!(contract.tier_deposit(3), 8);
}

/// Test if a function can only be called by the contract admin
#[ink::test]
fn onlyowner_works() {
    let contract = make_contract();
    assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
}

/// Test that we can transfer owner to another account
#[ink::test]
fn transfer_ownership_works() {
    let mut contract = make_contract();
    assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
    contract
        .transfer_ownership(AccountId::from([0x0; 32]))
        .unwrap();
    assert_eq!(contract.only_owner(AccountId::from([0x0; 32])), Ok(()));
}

/// Test the contract can take payment from users
#[ink::test]
fn subscribe_works() {
    let mut contract = make_contract();
    let payer = AccountId::from([0x1; 32]);
    assert_eq!(contract.balance_of(payer), 0);
    assert_eq!(contract.subscribe(3), Ok(()));

    let mut subscription = contract.subscriptions.get(&payer).unwrap();

    assert_eq!(subscription.end_date_ms, PERIOD_MS);
    assert_eq!(subscription.balance, 500);

    contract.subscribe(3).unwrap();

    subscription = contract.subscriptions.get(&payer).unwrap();

    assert_eq!(subscription.end_date_ms, PERIOD_MS * 2);
    assert_eq!(subscription.balance, 1000);

    // assert_eq!(contract.balance_of(payer), 2);
}

/// Test the total balance of the contract is correct
#[ink::test]
fn balance_of_contract_works() {
    let mut contract = make_contract();
    let payer_one = AccountId::from([0x1; 32]);
    assert_eq!(contract.balance_of(payer_one), 0);
    assert_eq!(contract.subscribe(3), Ok(()));
    assert_eq!(contract.balance_of_contract(), 0);
}

/// Test the contract can return the correct tier if given an account id
#[ink::test]
fn tier_id_of_works() {
    let mut contract = make_contract();
    let payer_one = AccountId::from([0x1; 32]);
    assert_eq!(contract.balance_of(payer_one), 0);
    assert_eq!(contract.subscribe(2), Ok(()));
    assert_eq!(contract.tier_id_of(payer_one), 2);
}

/// Test we can read metrics
#[ink::test]
fn get_all_tiers_works() {
    let contract = make_contract();

    let tiers = contract.get_all_tiers();
    assert_eq!(tiers[0].tier_id, 1);
    assert_eq!(tiers[0].tier_fee, 2);
    assert_eq!(tiers[0].storage_bytes, 2000);
    assert_eq!(tiers[0].wcu, 2000);
    assert_eq!(tiers[0].rcu, 2000);

    assert_eq!(tiers[1].tier_id, 2);
    assert_eq!(tiers[1].tier_fee, 4);
    assert_eq!(tiers[1].storage_bytes, 4000);
    assert_eq!(tiers[1].wcu, 4000);
    assert_eq!(tiers[1].rcu, 4000);

    assert_eq!(tiers[2].tier_id, 3);
    assert_eq!(tiers[2].tier_fee, 8);
    assert_eq!(tiers[2].storage_bytes, 8000);
    assert_eq!(tiers[2].wcu, 8000);
    assert_eq!(tiers[2].rcu, 8000);

}

/// Test the contract owner can change tier fees for all 3 tiers
#[ink::test]
fn change_tier_fee_works() {
    let mut contract = make_contract();
    assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
    assert_eq!(contract.change_tier_fee(3, 3), Ok(()));
    assert_eq!(contract.change_tier_fee(2, 5), Ok(()));
    assert_eq!(contract.change_tier_fee(1, 9), Ok(()));
    assert_eq!(contract.tier_deposit(3), 3);
    assert_eq!(contract.tier_deposit(2), 5);
    assert_eq!(contract.tier_deposit(1), 9);
}

/// Test the contract can change tier limits for all 3 tiers
#[ink::test]
fn change_tier_limit_works() {
    let mut contract = make_contract();
    assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
    assert_eq!(contract.change_tier_limit(3, 100, 100, 100), Ok(()));
    assert_eq!(contract.change_tier_limit(2, 200, 200, 200), Ok(()));
    assert_eq!(contract.change_tier_limit(1, 300, 300, 300), Ok(()));
    assert_eq!(contract.get_tier_limit(3), ServiceTier::new(3, 8, 100, 100, 100));
    assert_eq!(contract.get_tier_limit(2), ServiceTier::new(2, 4, 200, 200, 200));
    assert_eq!(contract.get_tier_limit(1), ServiceTier::new(1, 2, 300, 300, 300));
}

/// Test the contract owner can flip the status of the contract
/// Can pause and unpause the contract
#[ink::test]
fn flip_contract_status_works() {
    let mut contract = make_contract();
    assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
    assert_eq!(contract.paused_or_not(), false);
    assert_eq!(contract.flip_contract_status(), Ok(()));
    assert_eq!(contract.paused_or_not(), true);
    assert_eq!(contract.flip_contract_status(), Ok(()));
    assert_eq!(contract.paused_or_not(), false);
}

/// Test the contract owner can transfer all the balance out of the contract after it is paused
#[ink::test]
fn withdraw_works() {
    let mut contract = make_contract();
    let accounts = default_accounts::<DefaultEnvironment>().unwrap();

    // Endownment equivalence. Inititalize SC address with balance 1000
    set_balance(contract_id(), 1000);
    set_balance(accounts.bob, 0);
    assert_eq!(balance_of(contract_id()), 1000);

    // Non-owner cannot withdraw.
    set_caller(accounts.bob);
    assert_eq!(contract.withdraw(accounts.bob, 200), Err(OnlyOwner));
    assert_eq!(balance_of(contract_id()), 1000);
    undo_set_caller(); // Back to Alice owner.

    // Cannot withdraw to the zero account by mistake.
    assert_eq!(
        contract.withdraw(AccountId::default(), 200),
        Err(InvalidAccount)
    );

    // Cannot withdraw the entire balance by mistake.
    assert_eq!(
        contract.withdraw(accounts.bob, 1000),
        Err(InsufficientBalance)
    );

    // Can withdraw some tokens.
    assert_eq!(contract.withdraw(accounts.bob, 200), Ok(()));
    assert_eq!(balance_of(accounts.bob), 200);
    assert_eq!(balance_of(contract_id()), 800);
    assert_eq!(contract.balance_of_contract(), 800);
}

/// Sets the caller
fn set_caller(caller: AccountId) {
    let callee = ink_env::account_id::<ink_env::DefaultEnvironment>().unwrap_or([0x0; 32].into());
    test::push_execution_context::<Environment>(
        caller,
        callee,
        1000000,
        1000000,
        test::CallData::new(call::Selector::new([0x00; 4])), // dummy
    );
}

fn undo_set_caller() {
    test::pop_execution_context();
}

fn balance_of(account: AccountId) -> Balance {
    test::get_account_balance::<ink_env::DefaultEnvironment>(account).unwrap()
}

fn set_balance(account: AccountId, balance: Balance) {
    ink_env::test::set_account_balance::<ink_env::DefaultEnvironment>(account, balance).unwrap();
}

fn contract_id() -> AccountId {
    ink_env::test::get_current_contract_account_id::<ink_env::DefaultEnvironment>().unwrap()
}

#[ink::test]
fn get_median_works() {
    let vec = vec![7, 1, 7, 9999, 9, 7, 0];
    assert_eq!(get_median(vec), Some(7));
}

#[ink::test]
fn report_metrics_works() {
    let mut contract = make_contract();
    let accounts = default_accounts::<DefaultEnvironment>().unwrap();
    let reporter_id = accounts.alice;
    let app_id = accounts.charlie;

    let mut metrics = MetricValue {
        storage_bytes: 11,
        wcu_used: 12,
        rcu_used: 13,
        start_ms: 0,
    };
    let mut big_metrics = MetricValue {
        storage_bytes: 100,
        wcu_used: 101,
        rcu_used: 102,
        start_ms: 0,
    };
    let mut double_big_metrics = MetricValue {
        storage_bytes: 200,
        wcu_used: 202,
        rcu_used: 204,
        start_ms: 0,
    };
    // Note: the values of start_ms will be updated to use in assert_eq!

    let some_day = 9999;
    let period_start_ms = some_day / PERIOD_DAYS * PERIOD_MS;

    let today_ms = some_day * MS_PER_DAY; // Midnight time on some day.
    let today_key = MetricKey {
        reporter: reporter_id,
        app_id,
        day_of_period: some_day % PERIOD_DAYS,
    };

    let yesterday_ms = (some_day - 1) * MS_PER_DAY; // Midnight time on some day.
    let yesterday_key = MetricKey {
        reporter: reporter_id,
        app_id,
        day_of_period: (some_day - 1) % PERIOD_DAYS,
    };

    let next_month_ms = (some_day + PERIOD_DAYS) * MS_PER_DAY; // Midnight time on some day.
    let next_month_key = MetricKey {
        reporter: reporter_id,
        app_id,
        day_of_period: (some_day + PERIOD_DAYS) % PERIOD_DAYS,
    };

    // Unauthorized report, we are not a reporter.
    let err = contract.report_metrics(app_id, 0, metrics.storage_bytes, metrics.wcu_used, metrics.rcu_used);
    assert_eq!(err, Err(Error::OnlyReporter));

    // No metric yet.
    assert_eq!(contract.metrics.get(&today_key), None);
    assert_eq!(
        contract.metrics_for_period(app_id, 0, today_ms),
        MetricValue {
            start_ms: period_start_ms,
            storage_bytes: 0,
            wcu_used: 0,
            rcu_used: 0,
        }
    );

    // Authorize our admin account to be a reporter too.
    contract.add_reporter(reporter_id).unwrap();

    // Wrong day format.
    let err = contract.report_metrics(app_id, today_ms + 1, metrics.storage_bytes, metrics.wcu_used, metrics.rcu_used);
    assert_eq!(err, Err(Error::UnexpectedTimestamp));

    // Store metrics.
    contract
        .report_metrics(
            app_id,
            yesterday_ms,
            big_metrics.storage_bytes,
            big_metrics.wcu_used,
            big_metrics.rcu_used,
        )
        .unwrap();

    contract
        .report_metrics(app_id, today_ms, metrics.storage_bytes, metrics.wcu_used, metrics.rcu_used)
        .unwrap();

    big_metrics.start_ms = yesterday_ms;
    assert_eq!(contract.metrics.get(&yesterday_key), Some(&big_metrics));
    metrics.start_ms = today_ms;
    assert_eq!(contract.metrics.get(&today_key), Some(&metrics));

    // Update with bigger metrics.
    contract
        .report_metrics(
            app_id,
            today_ms,
            big_metrics.storage_bytes,
            big_metrics.wcu_used,
            big_metrics.rcu_used,
        )
        .unwrap();

    big_metrics.start_ms = today_ms;
    assert_eq!(contract.metrics.get(&today_key), Some(&big_metrics));

    // The metrics for the month is yesterday + today, both big_metrics now.
    double_big_metrics.start_ms = period_start_ms;
    assert_eq!(
        contract.metrics_for_period(app_id, period_start_ms, today_ms),
        double_big_metrics
    );
    double_big_metrics.start_ms = yesterday_ms;
    assert_eq!(
        contract.metrics_for_period(app_id, yesterday_ms, today_ms),
        double_big_metrics
    );

    // If the app start date was today, then its metrics would be only today.
    big_metrics.start_ms = today_ms;
    assert_eq!(
        contract.metrics_for_period(app_id, today_ms, today_ms),
        big_metrics
    );

    // Update one month later, overwriting the same day slot.
    assert_eq!(contract.metrics.get(&next_month_key), Some(&big_metrics));
    contract
        .report_metrics(
            app_id,
            next_month_ms,
            metrics.storage_bytes,
            metrics.wcu_used,
            metrics.rcu_used,
        )
        .unwrap();
    metrics.start_ms = next_month_ms;
    assert_eq!(contract.metrics.get(&next_month_key), Some(&metrics));

    // Some other account has no metrics.
    let other_key = MetricKey {
        reporter: reporter_id,
        app_id: accounts.bob,
        day_of_period: 0,
    };
    assert_eq!(contract.metrics.get(&other_key), None);
}

#[ink::test]
fn get_current_period_days_works() {
    const D: u64 = 10007; // A random day.
    let some_time = 12345;
    let another_time = 67890;

    let check = |subscription_day, period_day, now_day, number_of_days| {
        assert_eq!(
            get_current_period_days(
                subscription_day * MS_PER_DAY + some_time,
                now_day * MS_PER_DAY + another_time
            ),
            (period_day, now_day)
        );
        // Number of days between period start and now, both inclusive.
        assert_eq!(1 + now_day - period_day, number_of_days)
    };

    let is_first_day = 1;
    let two_days = 2;
    let full_period = PERIOD_DAYS;

    //    The subscription starts on day D.
    //    |  When the current period starts (same day as subscription, but in most recent month)
    //    |  |  The current day (included in the period)
    //    |  |  |    How many days are included in the period.
    check(D, D, D, is_first_day); // First day of the first period.
    check(D, D, D + 1, two_days);
    check(D, D, D + 30, full_period); // 31st day of the first period.

    check(D, D + 31, D + 31, is_first_day); // First day of the second period.
    check(D, D + 31, D + 31 + 1, two_days);
    check(D, D + 31, D + 31 + 30, full_period); // 31st day of the first period.

    check(D, D + 31 + 31, D + 31 + 31, is_first_day); // First day of the third period.
}

#[ink::test]
fn median_works() {
    let mut contract = make_contract();

    let alice = AccountId::from([0x01; 32]);
    let bob = AccountId::from([0x02; 32]);
    let charlie = AccountId::from([0x03; 32]);
    let django = AccountId::from([0x04; 32]);
    let eve = AccountId::from([0x05; 32]);
    let frank = AccountId::from([0x06; 32]);

    contract.add_reporter(alice).unwrap();
    contract.add_reporter(bob).unwrap();
    contract.add_reporter(charlie).unwrap();
    contract.add_reporter(django).unwrap();
    contract.add_reporter(eve).unwrap();
    contract.add_reporter(frank).unwrap();

    let day1 = 10001;
    let day1_ms = day1 * MS_PER_DAY;
    let day2 = 10002;
    let day2_ms = day2 * MS_PER_DAY;
    let day3 = 10003;
    let day3_ms = day3 * MS_PER_DAY;
    let day4 = 10004;
    let day4_ms = day4 * MS_PER_DAY;
    let day5 = 10005;
    let day5_ms = day5 * MS_PER_DAY;

    let day1_alice_django_key = MetricKey {
        reporter: alice,
        app_id: django,
        day_of_period: day1 % PERIOD_DAYS,
    };

    // No metric yet.
    assert_eq!(contract.metrics.get(&day1_alice_django_key), None);
    assert_eq!(
        contract.metrics_for_period(django, day1_ms, day5_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 0,
            wcu_used: 0,
            rcu_used: 0,
        }
    );

    // bob day1: [0, 6, 8, 8, 100] -> 8
    // bob day2: [2, 4, 4, 5, 6] -> 4
    // bob day3: [5, 8, 10, 11, 11] -> 10
    // bob day4: [8, 16, 20, 50, 80] -> 20
    // bob day5: [0, 0, 2, 2, 2] -> 2

    // charlie day1: [0, 1, 4, 5, 5] -> 4
    // charlie day2: [2, 4, 4, 5, 5] -> 4
    // charlie day3: [2, 2, 2, 11, 11] -> 2
    // charlie day4: [0, 4, 5, 5, 5] -> 5
    // charlie day5: [0, 0, 10, 11, 11]-> 10

    // django day1: [1, 1, 1, 1, 5] -> 1
    // django day2: [0, 5, 5, 5, 5] -> 5
    // django day3: [1, 8, 8, 8, 1000] -> 8
    // django day4: [2, 2, 10, 10] -> 2 ?
    // django day5: [2, 2, 2, 10] -> 2

    // eve day1: [5, 5, 5, 5] -> 5
    // eve day2: [1, 5, 5, 5] -> 5
    // eve day3: [1, 6, 6, 10] -> 6
    // eve day4: [2, 4, 6, 10] -> 4
    // eve day5: [1, 1, 1, 100] -> 1

    // frank day1: [7, 7, 7] -> 7
    // frank day2: [0, 10, 10] -> 10
    // frank day3: [2, 2, 10] -> 2
    // frank day4: [0, 10, 20] -> 10
    // frank day5: [1, 2, 3] -> 2

    // alice day1: [2, 5] -> 2
    // alice day2: [0, 10] -> 0
    // alice day3: [7, 7] -> 7
    // alice day4: [2] - 2
    // alice day5: [] - 0

    // Day 1
    set_caller(bob);
    contract.report_metrics(bob, day1_ms, 8, 1, 1).unwrap();
    contract.report_metrics(charlie, day1_ms, 0, 2, 2).unwrap();
    contract.report_metrics(django, day1_ms, 1, 3, 3).unwrap();
    contract.report_metrics(eve, day1_ms, 5, 4, 4).unwrap();
    contract.report_metrics(frank, day1_ms, 7, 5, 5).unwrap();
    contract.report_metrics(alice, day1_ms, 2, 6, 6).unwrap();
    undo_set_caller();

    set_caller(charlie);
    contract.report_metrics(bob, day1_ms, 6, 1, 1).unwrap();
    contract.report_metrics(charlie, day1_ms, 1, 2, 2).unwrap();
    contract.report_metrics(django, day1_ms, 1, 3, 3).unwrap();
    contract.report_metrics(eve, day1_ms, 5, 4, 4).unwrap();
    undo_set_caller();

    set_caller(django);
    contract.report_metrics(bob, day1_ms, 8, 1, 1).unwrap();
    contract.report_metrics(charlie, day1_ms, 4, 2, 2).unwrap();
    contract.report_metrics(django, day1_ms, 5, 3, 3).unwrap();
    contract.report_metrics(eve, day1_ms, 5, 4, 4).unwrap();
    contract.report_metrics(frank, day1_ms, 7, 5, 5).unwrap();
    contract.report_metrics(alice, day1_ms, 5, 6, 6).unwrap();
    undo_set_caller();

    set_caller(eve);
    contract.report_metrics(bob, day1_ms, 0, 1, 1).unwrap();
    contract.report_metrics(charlie, day1_ms, 5, 2, 2).unwrap();
    contract.report_metrics(django, day1_ms, 1, 3, 3).unwrap();
    contract.report_metrics(eve, day1_ms, 5, 4, 4).unwrap();
    contract.report_metrics(frank, day1_ms, 7, 5, 5).unwrap();

    undo_set_caller();

    set_caller(frank);
    contract.report_metrics(bob, day1_ms, 100, 1, 1).unwrap();
    contract.report_metrics(charlie, day1_ms, 5, 2, 2).unwrap();
    contract.report_metrics(django, day1_ms, 1, 3, 3).unwrap();
    undo_set_caller();

    // Day 2
    set_caller(bob);
    contract.report_metrics(bob, day2_ms, 2, 1, 1).unwrap();
    contract.report_metrics(charlie, day2_ms, 5, 2, 2).unwrap();
    contract.report_metrics(django, day2_ms, 5, 3, 3).unwrap();
    contract.report_metrics(eve, day2_ms, 5, 4, 4).unwrap();
    contract.report_metrics(frank, day2_ms, 0, 5, 5).unwrap();
    contract.report_metrics(alice, day2_ms, 0, 6, 6).unwrap();
    undo_set_caller();

    set_caller(charlie);
    contract.report_metrics(bob, day2_ms, 4, 1, 1).unwrap();
    contract.report_metrics(charlie, day2_ms, 5, 2, 2).unwrap();
    contract.report_metrics(django, day2_ms, 0, 3, 3).unwrap();
    contract.report_metrics(eve, day2_ms, 1, 4, 4).unwrap();
    contract.report_metrics(frank, day2_ms, 10, 5, 5).unwrap();
    undo_set_caller();

    set_caller(django);
    contract.report_metrics(bob, day2_ms, 5, 1, 1).unwrap();
    contract.report_metrics(charlie, day2_ms, 4, 2, 2).unwrap();
    contract.report_metrics(django, day2_ms, 5, 3, 3).unwrap();
    contract.report_metrics(eve, day2_ms, 5, 4, 4).unwrap();
    contract.report_metrics(frank, day2_ms, 10, 5, 5).unwrap();
    contract.report_metrics(alice, day2_ms, 10, 6, 6).unwrap();
    undo_set_caller();

    set_caller(eve);
    contract.report_metrics(bob, day2_ms, 6, 1, 1).unwrap();
    contract.report_metrics(charlie, day2_ms, 4, 2, 2).unwrap();
    contract.report_metrics(django, day2_ms, 5, 3, 3).unwrap();
    contract.report_metrics(eve, day2_ms, 5, 4, 4).unwrap();
    undo_set_caller();

    set_caller(frank);
    contract.report_metrics(bob, day2_ms, 4, 1, 1).unwrap();
    contract.report_metrics(charlie, day2_ms, 2, 2, 2).unwrap();
    contract.report_metrics(django, day2_ms, 5, 3, 3).unwrap();
    undo_set_caller();

    // Day3
    set_caller(bob);
    contract.report_metrics(bob, day3_ms, 11, 1, 1).unwrap();
    contract.report_metrics(charlie, day3_ms, 11, 2, 2).unwrap();
    contract.report_metrics(django, day3_ms, 1000, 3, 3).unwrap();
    contract.report_metrics(eve, day3_ms, 1, 4, 4).unwrap();
    contract.report_metrics(frank, day3_ms, 10, 5, 5).unwrap();
    contract.report_metrics(alice, day3_ms, 7, 6, 6).unwrap();
    undo_set_caller();

    set_caller(charlie);
    contract.report_metrics(bob, day3_ms, 11, 1, 1).unwrap();
    contract.report_metrics(charlie, day3_ms, 2, 2, 2).unwrap();
    contract.report_metrics(django, day3_ms, 8, 3, 3).unwrap();
    contract.report_metrics(eve, day3_ms, 6, 4, 4).unwrap();
    undo_set_caller();

    set_caller(django);
    contract.report_metrics(bob, day3_ms, 8, 1, 1).unwrap();
    contract.report_metrics(charlie, day3_ms, 11, 2, 2).unwrap();
    contract.report_metrics(django, day3_ms, 8, 3, 3).unwrap();
    contract.report_metrics(eve, day3_ms, 6, 4, 4).unwrap();
    contract.report_metrics(frank, day3_ms, 2, 5, 5).unwrap();
    contract.report_metrics(alice, day3_ms, 7, 6, 6).unwrap();
    undo_set_caller();

    set_caller(eve);
    contract.report_metrics(bob, day3_ms, 10, 1, 1).unwrap();
    contract.report_metrics(charlie, day3_ms, 2, 2, 2).unwrap();
    contract.report_metrics(django, day3_ms, 8, 3, 3).unwrap();
    contract.report_metrics(frank, day3_ms, 2, 5, 5).unwrap();
    undo_set_caller();

    set_caller(frank);
    contract.report_metrics(bob, day3_ms, 5, 1, 1).unwrap();
    contract.report_metrics(charlie, day3_ms, 2, 2, 2).unwrap();
    contract.report_metrics(django, day3_ms, 1, 3, 3).unwrap();
    contract.report_metrics(eve, day3_ms, 10, 4, 4).unwrap();
    undo_set_caller();

    // Day 4
    set_caller(bob);
    contract.report_metrics(bob, day4_ms, 80, 1, 1).unwrap();
    contract.report_metrics(charlie, day4_ms, 5, 2, 2).unwrap();
    contract.report_metrics(django, day4_ms, 10, 3, 3).unwrap();
    contract.report_metrics(frank, day4_ms, 20, 5, 5).unwrap();
    contract.report_metrics(alice, day4_ms, 2, 6, 6).unwrap();
    undo_set_caller();

    set_caller(charlie);
    contract.report_metrics(bob, day4_ms, 20, 1, 1).unwrap();
    contract.report_metrics(charlie, day4_ms, 0, 2, 2).unwrap();
    contract.report_metrics(django, day4_ms, 2, 3, 3).unwrap();
    contract.report_metrics(eve, day4_ms, 2, 4, 4).unwrap();
    contract.report_metrics(frank, day4_ms, 10, 5, 5).unwrap();
    undo_set_caller();

    set_caller(django);
    contract.report_metrics(bob, day4_ms, 50, 1, 1).unwrap();
    contract.report_metrics(charlie, day4_ms, 5, 2, 2).unwrap();
    contract.report_metrics(django, day4_ms, 10, 3, 3).unwrap();
    contract.report_metrics(eve, day4_ms, 4, 4, 4).unwrap();
    contract.report_metrics(frank, day4_ms, 0, 5, 5).unwrap();
    undo_set_caller();

    set_caller(eve);
    contract.report_metrics(bob, day4_ms, 8, 1, 1).unwrap();
    contract.report_metrics(charlie, day4_ms, 5, 2, 2).unwrap();
    contract.report_metrics(django, day4_ms, 2, 3, 3).unwrap();
    contract.report_metrics(eve, day4_ms, 6, 4, 4).unwrap();
    undo_set_caller();

    set_caller(frank);
    contract.report_metrics(bob, day4_ms, 16, 1, 1).unwrap();
    contract.report_metrics(charlie, day4_ms, 4, 2, 2).unwrap();
    contract.report_metrics(eve, day4_ms, 10, 4, 4).unwrap();
    undo_set_caller();

    // Day 5
    set_caller(bob);
    contract.report_metrics(bob, day5_ms, 2, 1, 1).unwrap();
    contract.report_metrics(charlie, day5_ms, 11, 2, 2).unwrap();
    contract.report_metrics(django, day5_ms, 10, 3, 3).unwrap();
    contract.report_metrics(eve, day5_ms, 1, 4, 4).unwrap();
    contract.report_metrics(frank, day5_ms, 1, 5, 5).unwrap();
    undo_set_caller();

    set_caller(charlie);
    contract.report_metrics(bob, day5_ms, 0, 1, 1).unwrap();
    contract.report_metrics(charlie, day5_ms, 10, 2, 2).unwrap();
    contract.report_metrics(django, day5_ms, 2, 3, 3).unwrap();
    contract.report_metrics(frank, day5_ms, 2, 5, 5).unwrap();
    undo_set_caller();

    set_caller(django);
    contract.report_metrics(bob, day5_ms, 0, 1, 1).unwrap();
    contract.report_metrics(charlie, day5_ms, 11, 2, 2).unwrap();
    contract.report_metrics(django, day5_ms, 2, 3, 3).unwrap();
    contract.report_metrics(eve, day5_ms, 100, 4, 5).unwrap();
    contract.report_metrics(frank, day5_ms, 3, 5, 5).unwrap();
    undo_set_caller();

    set_caller(eve);
    contract.report_metrics(bob, day5_ms, 2, 1, 1).unwrap();
    contract.report_metrics(charlie, day5_ms, 0, 2, 2).unwrap();
    contract.report_metrics(django, day5_ms, 2, 3, 3).unwrap();
    contract.report_metrics(eve, day5_ms, 1, 4, 4).unwrap();
    undo_set_caller();

    set_caller(frank);
    contract.report_metrics(bob, day5_ms, 2, 1, 1).unwrap();
    contract.report_metrics(charlie, day5_ms, 0, 2, 2).unwrap();
    contract.report_metrics(eve, day5_ms, 1, 4, 4).unwrap();
    undo_set_caller();

    // Bob
    assert_eq!(
        contract.metrics_for_period(bob, day1_ms, day1_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 8,
            wcu_used: 1,
            rcu_used: 1,
        }
    );
    assert_eq!(
        contract.metrics_for_period(bob, day2_ms, day2_ms),
        MetricValue {
            start_ms: day2_ms,
            storage_bytes: 4,
            wcu_used: 1,
            rcu_used: 1,
        }
    );
    assert_eq!(
        contract.metrics_for_period(bob, day3_ms, day3_ms),
        MetricValue {
            start_ms: day3_ms,
            storage_bytes: 10,
            wcu_used: 1,
            rcu_used: 1,
        }
    );
    assert_eq!(
        contract.metrics_for_period(bob, day4_ms, day4_ms),
        MetricValue {
            start_ms: day4_ms,
            storage_bytes: 20,
            wcu_used: 1,
            rcu_used: 1,
        }
    );
    assert_eq!(
        contract.metrics_for_period(bob, day5_ms, day5_ms),
        MetricValue {
            start_ms: day5_ms,
            storage_bytes: 2,
            wcu_used: 1,
            rcu_used: 1,
        }
    );

    assert_eq!(
        contract.metrics_for_period(bob, day1_ms, day5_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 44,
            wcu_used: 5,
            rcu_used: 5,
        }
    );
    assert_eq!(
        contract.metrics_for_period(bob, day1_ms, day2_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 12,
            wcu_used: 2,
            rcu_used: 2,
        }
    );
    assert_eq!(
        contract.metrics_for_period(bob, day1_ms, day3_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 22,
            wcu_used: 3,
            rcu_used: 3,
        }
    );
    assert_eq!(
        contract.metrics_for_period(bob, day2_ms, day5_ms),
        MetricValue {
            start_ms: day2_ms,
            storage_bytes: 36,
            wcu_used: 4,
            rcu_used: 4,
        }
    );

    // Charlie
    assert_eq!(
        contract.metrics_for_period(charlie, day1_ms, day1_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 4,
            wcu_used: 2,
            rcu_used: 2,
        }
    );
    assert_eq!(
        contract.metrics_for_period(charlie, day2_ms, day2_ms),
        MetricValue {
            start_ms: day2_ms,
            storage_bytes: 4,
            wcu_used: 2,
            rcu_used: 2,
        }
    );
    assert_eq!(
        contract.metrics_for_period(charlie, day3_ms, day3_ms),
        MetricValue {
            start_ms: day3_ms,
            storage_bytes: 2,
            wcu_used: 2,
            rcu_used: 2,
        }
    );
    assert_eq!(
        contract.metrics_for_period(charlie, day4_ms, day4_ms),
        MetricValue {
            start_ms: day4_ms,
            storage_bytes: 5,
            wcu_used: 2,
            rcu_used: 2,
        }
    );
    assert_eq!(
        contract.metrics_for_period(charlie, day5_ms, day5_ms),
        MetricValue {
            start_ms: day5_ms,
            storage_bytes: 10,
            wcu_used: 2,
            rcu_used: 2,
        }
    );

    assert_eq!(
        contract.metrics_for_period(charlie, day1_ms, day5_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 25,
            wcu_used: 10,
            rcu_used: 10,
        }
    );
    assert_eq!(
        contract.metrics_for_period(charlie, day1_ms, day2_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 8,
            wcu_used: 4,
            rcu_used: 4,
        }
    );
    assert_eq!(
        contract.metrics_for_period(charlie, day1_ms, day3_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 10,
            wcu_used: 6,
            rcu_used: 6,
        }
    );
    assert_eq!(
        contract.metrics_for_period(charlie, day2_ms, day5_ms),
        MetricValue {
            start_ms: day2_ms,
            storage_bytes: 21,
            wcu_used: 8,
            rcu_used: 8,
        }
    );

    // Django
    assert_eq!(
        contract.metrics_for_period(django, day1_ms, day1_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 1,
            wcu_used: 3,
            rcu_used: 3,
        }
    );
    assert_eq!(
        contract.metrics_for_period(django, day2_ms, day2_ms),
        MetricValue {
            start_ms: day2_ms,
            storage_bytes: 5,
            wcu_used: 3,
            rcu_used: 3,
        }
    );
    assert_eq!(
        contract.metrics_for_period(django, day3_ms, day3_ms),
        MetricValue {
            start_ms: day3_ms,
            storage_bytes: 8,
            wcu_used: 3,
            rcu_used: 3,
        }
    );
    assert_eq!(
        contract.metrics_for_period(django, day4_ms, day4_ms),
        MetricValue {
            start_ms: day4_ms,
            storage_bytes: 2,
            wcu_used: 3,
            rcu_used: 3,
        }
    );
    assert_eq!(
        contract.metrics_for_period(django, day5_ms, day5_ms),
        MetricValue {
            start_ms: day5_ms,
            storage_bytes: 2,
            wcu_used: 3,
            rcu_used: 3,
        }
    );

    assert_eq!(
        contract.metrics_for_period(django, day1_ms, day5_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 18,
            wcu_used: 15,
            rcu_used: 15,
        }
    );
    assert_eq!(
        contract.metrics_for_period(django, day1_ms, day2_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 6,
            wcu_used: 6,
            rcu_used: 6,
        }
    );
    assert_eq!(
        contract.metrics_for_period(django, day1_ms, day3_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 14,
            wcu_used: 9,
            rcu_used: 9,
        }
    );
    assert_eq!(
        contract.metrics_for_period(django, day2_ms, day5_ms),
        MetricValue {
            start_ms: day2_ms,
            storage_bytes: 17,
            wcu_used: 12,
            rcu_used: 12,
        }
    );

    // Eve
    assert_eq!(
        contract.metrics_for_period(eve, day1_ms, day1_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 5,
            wcu_used: 4,
            rcu_used: 4,
        }
    );
    assert_eq!(
        contract.metrics_for_period(eve, day2_ms, day2_ms),
        MetricValue {
            start_ms: day2_ms,
            storage_bytes: 5,
            wcu_used: 4,
            rcu_used: 4,
        }
    );
    assert_eq!(
        contract.metrics_for_period(eve, day3_ms, day3_ms),
        MetricValue {
            start_ms: day3_ms,
            storage_bytes: 6,
            wcu_used: 4,
            rcu_used: 4,
        }
    );
    assert_eq!(
        contract.metrics_for_period(eve, day4_ms, day4_ms),
        MetricValue {
            start_ms: day4_ms,
            storage_bytes: 4,
            wcu_used: 4,
            rcu_used: 4,
        }
    );
    assert_eq!(
        contract.metrics_for_period(eve, day5_ms, day5_ms),
        MetricValue {
            start_ms: day5_ms,
            storage_bytes: 1,
            wcu_used: 4,
            rcu_used: 4,
        }
    );

    assert_eq!(
        contract.metrics_for_period(eve, day1_ms, day5_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 21,
            wcu_used: 20,
            rcu_used: 20,
        }
    );
    assert_eq!(
        contract.metrics_for_period(eve, day1_ms, day2_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 10,
            wcu_used: 8,
            rcu_used: 8,
        }
    );
    assert_eq!(
        contract.metrics_for_period(eve, day1_ms, day3_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 16,
            wcu_used: 12,
            rcu_used: 12,
        }
    );
    assert_eq!(
        contract.metrics_for_period(eve, day2_ms, day5_ms),
        MetricValue {
            start_ms: day2_ms,
            storage_bytes: 16,
            wcu_used: 16,
            rcu_used: 16,
        }
    );

    // Frank
    assert_eq!(
        contract.metrics_for_period(frank, day1_ms, day1_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 7,
            wcu_used: 5,
            rcu_used: 5,
        }
    );
    assert_eq!(
        contract.metrics_for_period(frank, day2_ms, day2_ms),
        MetricValue {
            start_ms: day2_ms,
            storage_bytes: 10,
            wcu_used: 5,
            rcu_used: 5,
        }
    );
    assert_eq!(
        contract.metrics_for_period(frank, day3_ms, day3_ms),
        MetricValue {
            start_ms: day3_ms,
            storage_bytes: 2,
            wcu_used: 5,
            rcu_used: 5,
        }
    );
    assert_eq!(
        contract.metrics_for_period(frank, day4_ms, day4_ms),
        MetricValue {
            start_ms: day4_ms,
            storage_bytes: 10,
            wcu_used: 5,
            rcu_used: 5,
        }
    );
    assert_eq!(
        contract.metrics_for_period(frank, day5_ms, day5_ms),
        MetricValue {
            start_ms: day5_ms,
            storage_bytes: 2,
            wcu_used: 5,
            rcu_used: 5,
        }
    );

    assert_eq!(
        contract.metrics_for_period(frank, day1_ms, day5_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 31,
            wcu_used: 25,
            rcu_used: 25,
        }
    );
    assert_eq!(
        contract.metrics_for_period(frank, day1_ms, day2_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 17,
            wcu_used: 10,
            rcu_used: 10,
        }
    );
    assert_eq!(
        contract.metrics_for_period(frank, day1_ms, day3_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 19,
            wcu_used: 15,
            rcu_used: 15,
        }
    );
    assert_eq!(
        contract.metrics_for_period(frank, day2_ms, day5_ms),
        MetricValue {
            start_ms: day2_ms,
            storage_bytes: 24,
            wcu_used: 20,
            rcu_used: 20,
        }
    );

    // Alice
    assert_eq!(
        contract.metrics_for_period(alice, day1_ms, day1_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 2,
            wcu_used: 6,
            rcu_used: 6,
        }
    );
    assert_eq!(
        contract.metrics_for_period(alice, day2_ms, day2_ms),
        MetricValue {
            start_ms: day2_ms,
            storage_bytes: 0,
            wcu_used: 6,
            rcu_used: 6,
        }
    );
    assert_eq!(
        contract.metrics_for_period(alice, day3_ms, day3_ms),
        MetricValue {
            start_ms: day3_ms,
            storage_bytes: 7,
            wcu_used: 6,
            rcu_used: 6,
        }
    );
    assert_eq!(
        contract.metrics_for_period(alice, day4_ms, day4_ms),
        MetricValue {
            start_ms: day4_ms,
            storage_bytes: 2,
            wcu_used: 6,
            rcu_used: 6,
        }
    );
    // no metrics
    assert_eq!(
        contract.metrics_for_period(alice, day5_ms, day5_ms),
        MetricValue {
            start_ms: day5_ms,
            storage_bytes: 0,
            wcu_used: 0,
            rcu_used: 0,
        }
    );

    assert_eq!(
        contract.metrics_for_period(alice, day1_ms, day5_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 11,
            wcu_used: 24,
            rcu_used: 24,
        }
    );
    assert_eq!(
        contract.metrics_for_period(alice, day1_ms, day2_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 2,
            wcu_used: 12,
            rcu_used: 12,
        }
    );
    assert_eq!(
        contract.metrics_for_period(alice, day1_ms, day3_ms),
        MetricValue {
            start_ms: day1_ms,
            storage_bytes: 9,
            rcu_used: 18,
            wcu_used: 18,
        }
    );
    assert_eq!(
        contract.metrics_for_period(alice, day2_ms, day5_ms),
        MetricValue {
            start_ms: day2_ms,
            storage_bytes: 9,
            wcu_used: 18,
            rcu_used: 18,
        }
    );
}

#[ink::test]
fn metrics_since_subscription_works() {
    let mut contract = make_contract();
    let accounts = default_accounts::<DefaultEnvironment>().unwrap();
    let app_id = accounts.charlie;

    // No subscription yet.
    assert_eq!(
        contract.metrics_since_subscription(app_id),
        Err(Error::NoSubscription)
    );

    // Charlie subscribes for her app. The start date will be 0.
    set_caller(app_id);
    contract.subscribe(1).unwrap();
    undo_set_caller(); // Back to Alice admin.

    // Subscription without metrics.
    assert_eq!(
        contract.metrics_since_subscription(app_id),
        Ok(MetricValue {
            start_ms: 0,
            storage_bytes: 0,
            wcu_used: 0,
            rcu_used: 0,
        })
    );

    // Subscription with metrics.
    contract.add_reporter(accounts.alice).unwrap();
    contract.report_metrics(app_id, 0, 12, 34, 34).unwrap();
    assert_eq!(
        contract.metrics_since_subscription(app_id),
        Ok(MetricValue {
            start_ms: 0,
            storage_bytes: 12,
            wcu_used: 34,
            rcu_used: 34,
        })
    );
}

#[ink::test]
fn finalize_metric_period_works() {
    let mut contract = make_contract();
    let accounts = default_accounts::<DefaultEnvironment>().unwrap();
    let yesterday_ms = 9999 * MS_PER_DAY; // Midnight time on some day.
    let today_ms = yesterday_ms + MS_PER_DAY;

    // Unauthorized report, we are not a reporter.
    let err = contract.finalize_metric_period(yesterday_ms);
    assert_eq!(err, Err(Error::OnlyReporter));

    // Authorize our admin account to be a reporter too.
    contract.add_reporter(accounts.alice).unwrap();

    // Wrong day format.
    let err = contract.finalize_metric_period(yesterday_ms + 1);
    assert_eq!(err, Err(Error::UnexpectedTimestamp));

    // Finalize today to change the current period.
    assert_eq!(contract.get_current_period_ms(), 0);
    contract.finalize_metric_period(yesterday_ms).unwrap();
    assert_eq!(contract.get_current_period_ms(), today_ms);
}

#[ink::test]
fn get_current_period_ms_works() {
    let mut contract = make_contract();
    let accounts = default_accounts::<DefaultEnvironment>().unwrap();
    let day0 = 9999 * MS_PER_DAY; // Midnight time on some day.
    let day1 = day0 + MS_PER_DAY;
    let day2 = day1 + MS_PER_DAY;

    // Authorize our accounts to be a reporters.
    contract.add_reporter(accounts.alice).unwrap();
    contract.add_reporter(accounts.bob).unwrap();

    // Initial values are the current day (0 because that is the current time in the test env).
    assert_eq!(contract.get_current_period_ms_of(accounts.alice), 0);
    assert_eq!(contract.get_current_period_ms_of(accounts.bob), 0);
    assert_eq!(contract.get_current_period_ms(), 0); // of caller Alice

    // Alice finalizes day 0.
    contract.finalize_metric_period(day0).unwrap();
    assert_eq!(contract.get_current_period_ms_of(accounts.alice), day1); // After day0.
    assert_eq!(contract.get_current_period_ms_of(accounts.bob), 0); // No change.
    assert_eq!(contract.get_current_period_ms(), day1); // of caller Alice

    // Bob finalizes day 1.
    set_caller(accounts.bob);
    contract.finalize_metric_period(day1).unwrap();
    assert_eq!(contract.get_current_period_ms_of(accounts.alice), day1); // No change.
    assert_eq!(contract.get_current_period_ms_of(accounts.bob), day2); // After day1.
    assert_eq!(contract.get_current_period_ms(), day2); // of caller Bob
    undo_set_caller();

    // Alice finalizes day 1.
    contract.finalize_metric_period(day1).unwrap();
    assert_eq!(contract.get_current_period_ms_of(accounts.alice), day2); // After day1.
    assert_eq!(contract.get_current_period_ms_of(accounts.bob), day2); // No change.
    assert_eq!(contract.get_current_period_ms(), day2); // of caller Alice
}

fn decode_event(event: &ink_env::test::EmittedEvent) -> Event {
    <Event as scale::Decode>::decode(&mut &event.data[..])
        .expect("encountered invalid contract event data buffer")
}

// ---- Admin: Reporters ----
#[ink::test]
fn add_and_remove_reporters_works() {
    let mut contract = make_contract();

    let new_reporter = AccountId::from([0x1; 32]);

    assert!(!contract.is_reporter(new_reporter));
    contract.add_reporter(new_reporter).unwrap();
    assert!(contract.is_reporter(new_reporter));
    contract.remove_reporter(new_reporter).unwrap();
    assert!(!contract.is_reporter(new_reporter));

    let raw_events = recorded_events().collect::<Vec<_>>();
    assert_eq!(5, raw_events.len()); // 3 x tier added + added reporter + remove reporter

    if let Event::ReporterAdded(ReporterAdded { reporter }) = decode_event(&raw_events[3]) {
        assert_eq!(reporter, new_reporter);
    } else {
        panic!("Wrong event type");
    }

    if let Event::ReporterRemoved(ReporterRemoved { reporter }) =
        decode_event(&raw_events[4])
    {
        assert_eq!(reporter, new_reporter);
    } else {
        panic!("Wrong event type");
    }
}

// ---- DDC Nodes ----
#[ink::test]
fn get_all_ddc_nodes_works() {
    let contract = make_contract();

    // Return an empty list
    assert_eq!(contract.get_all_ddc_nodes(), vec![]);
}

#[ink::test]
fn add_ddc_node_only_owner_works() {
    let mut contract = make_contract();
    let accounts = default_accounts::<DefaultEnvironment>().unwrap();
    let p2p_id = String::from("test_p2p_id");
    let url = String::from("test_url");

    // Should be an owner
    set_caller(accounts.charlie);
    assert_eq!(contract.add_ddc_node(p2p_id, url), Err(Error::OnlyOwner));
}

#[ink::test]
fn add_ddc_node_works() {
    let mut contract = make_contract();
    let p2p_id = String::from("test_p2p_id");
    let url = String::from("test_url");

    // Add DDC node to the list
    contract.add_ddc_node(p2p_id.clone(), url.clone()).unwrap();

    // Should be in the list
    assert_eq!(
        contract.get_all_ddc_nodes(),
        vec![DDCNode {
            p2p_id: p2p_id.clone(),
            url: url.clone()
        },]
    );

    // Should add the default DDN status
    assert_eq!(
        contract.get_ddn_status(p2p_id.clone()).unwrap(),
        DDNStatus {
            is_online: true,
            total_downtime: 0,
            reference_timestamp: 0,
            last_timestamp: 0,
        }
    );

    // Should emit event
    let raw_events = recorded_events().collect::<Vec<_>>();
    assert_eq!(4, raw_events.len()); // 3 x tier added + node added
    if let Event::DDCNodeAdded(DDCNodeAdded {
        p2p_id: event_p2p_id,
        url: event_url,
    }) = decode_event(&raw_events[3])
    {
        assert_eq!(event_p2p_id, p2p_id);
        assert_eq!(event_url, url);
    } else {
        panic!("Wrong event type")
    }
}

#[ink::test]
fn add_ddn_node_update_url_works() {
    let mut contract = make_contract();
    let p2p_id = String::from("test_p2p_id");
    let url = String::from("test_url");
    let new_url = String::from("test_url_new");

    // Add DDC node to the list
    contract.add_ddc_node(p2p_id.clone(), url.clone()).unwrap();

    // Update DDC node url
    contract
        .add_ddc_node(p2p_id.clone(), new_url.clone())
        .unwrap();

    // Get the list of DDC nodes
    assert_eq!(
        contract.get_all_ddc_nodes(),
        vec![DDCNode {
            p2p_id,
            url: new_url
        }]
    );
}

#[ink::test]
fn is_ddc_node_works() {
    let mut contract = make_contract();
    let p2p_id = String::from("test_p2p_id");
    let url = String::from("test_url");

    // Return false if not added
    assert_eq!(contract.is_ddc_node(p2p_id.clone()), false);

    // Add DDC node to the list
    contract.add_ddc_node(p2p_id.clone(), url.clone()).unwrap();

    // Should be in the list
    assert_eq!(contract.is_ddc_node(p2p_id), true);
}

#[ink::test]
fn remove_ddc_node_only_owner_works() {
    let mut contract = make_contract();
    let accounts = default_accounts::<DefaultEnvironment>().unwrap();
    let p2p_id = String::from("test_p2p_id");

    // Should be an owner
    set_caller(accounts.charlie);
    assert_eq!(contract.remove_ddc_node(p2p_id), Err(Error::OnlyOwner));
}

#[ink::test]
fn remove_ddc_node_works() {
    let mut contract = make_contract();
    let p2p_id = String::from("test_p2p_id");
    let url = String::from("test_url");

    // Add DDC node to the list
    contract.add_ddc_node(p2p_id.clone(), url.clone()).unwrap();

    // Remove DDC node
    contract.remove_ddc_node(p2p_id.clone()).unwrap();

    // Should be removed from the list
    assert_eq!(contract.get_all_ddc_nodes(), vec![]);

    // Should emit event
    let raw_events = recorded_events().collect::<Vec<_>>();
    assert_eq!(5, raw_events.len());
    if let Event::DDCNodeRemoved(DDCNodeRemoved {
        p2p_id: event_p2p_id,
    }) = decode_event(&raw_events[4])
    {
        assert_eq!(event_p2p_id, p2p_id);
    } else {
        panic!("Wrong event type")
    }
}

// ---- DDN Statuses ----

#[ink::test]
fn set_ddn_status_not_found_works() {
    let mut contract = make_contract();
    let p2p_id = String::from("test_p2p_id");

    // DDC node should be in the list
    assert_eq!(
        contract.set_ddn_status(p2p_id, 4, true),
        Err(Error::DDNNotFound)
    );
}

#[ink::test]
fn set_ddn_status_works() {
    let mut contract = make_contract();
    let p2p_id = String::from("test_p2p_id");
    let url = String::from("test_url");

    // Add DDC node to the list
    contract.add_ddc_node(p2p_id.clone(), url).unwrap();

    // Calculations should work
    contract.set_ddn_status(p2p_id.clone(), 4, true).unwrap();
    assert_eq!(
        contract.get_ddn_status(p2p_id.clone()).unwrap(),
        DDNStatus {
            is_online: true,
            total_downtime: 0,
            reference_timestamp: 0,
            last_timestamp: 4,
        }
    );

    contract.set_ddn_status(p2p_id.clone(), 6, true).unwrap();
    assert_eq!(
        contract.get_ddn_status(p2p_id.clone()).unwrap(),
        DDNStatus {
            is_online: true,
            total_downtime: 0,
            reference_timestamp: 0,
            last_timestamp: 6,
        }
    );

    contract.set_ddn_status(p2p_id.clone(), 8, false).unwrap();
    assert_eq!(
        contract.get_ddn_status(p2p_id.clone()),
        Ok(DDNStatus {
            is_online: false,
            total_downtime: 0,
            reference_timestamp: 0,
            last_timestamp: 8,
        })
    );

    contract.set_ddn_status(p2p_id.clone(), 10, false).unwrap();
    assert_eq!(
        contract.get_ddn_status(p2p_id.clone()),
        Ok(DDNStatus {
            is_online: false,
            total_downtime: 2,
            reference_timestamp: 0,
            last_timestamp: 10,
        })
    );

    contract.set_ddn_status(p2p_id.clone(), 12, true).unwrap();
    assert_eq!(
        contract.get_ddn_status(p2p_id.clone()),
        Ok(DDNStatus {
            is_online: true,
            total_downtime: 4,
            reference_timestamp: 0,
            last_timestamp: 12,
        })
    );

    contract.set_ddn_status(p2p_id.clone(), 18, false).unwrap();
    assert_eq!(
        contract.get_ddn_status(p2p_id.clone()),
        Ok(DDNStatus {
            is_online: false,
            total_downtime: 4,
            reference_timestamp: 0,
            last_timestamp: 18,
        })
    );

    contract.set_ddn_status(p2p_id.clone(), 25, true).unwrap();
    assert_eq!(
        contract.get_ddn_status(p2p_id.clone()),
        Ok(DDNStatus {
            is_online: true,
            total_downtime: 11,
            reference_timestamp: 0,
            last_timestamp: 25,
        })
    );
}

#[ink::test]
fn set_ddn_status_unexpected_timestamp_works() {
    let mut contract = make_contract();
    let p2p_id = String::from("test_p2p_id");
    let url = String::from("test_url");

    // Add DDC node to the list
    contract.add_ddc_node(p2p_id.clone(), url).unwrap();

    // Set status for a timestamp
    assert_eq!(contract.set_ddn_status(p2p_id.clone(), 10, true), Ok(()));

    // Specified timestamp must be greater than the last one
    assert_eq!(
        contract.set_ddn_status(p2p_id, 8, true),
        Err(Error::UnexpectedTimestamp)
    );
}

#[ink::test]
fn get_ddn_status_not_found_works() {
    let contract = make_contract();
    let p2p_id = String::from("test_p2p_id");

    // Should return an error if not found
    assert_eq!(contract.get_ddn_status(p2p_id), Err(Error::DDNNotFound));
}

#[ink::test]
fn get_ddn_status_works() {
    let mut contract = make_contract();
    let p2p_id = String::from("test_p2p_id");
    let url = String::from("test_url");

    // Add DDC node to the list
    contract.add_ddc_node(p2p_id.clone(), url).unwrap();

    // Set new status
    contract.set_ddn_status(p2p_id.clone(), 2, false).unwrap();

    // Get updated status
    assert_eq!(
        contract.get_ddn_status(p2p_id),
        Ok(DDNStatus {
            is_online: false,
            total_downtime: 0,
            reference_timestamp: 0,
            last_timestamp: 2,
        })
    );
}

#[ink::test]
fn report_ddn_status_only_reporter_works() {
    let mut contract = make_contract();
    let p2p_id = String::from("test_p2p_id");

    // Caller should be a reporter
    assert_eq!(
        contract.report_ddn_status(p2p_id.clone(), true),
        Err(Error::OnlyReporter)
    );
}

#[ink::test]
fn report_ddn_status_not_found_works() {
    let mut contract = make_contract();
    let accounts = default_accounts::<DefaultEnvironment>().unwrap();
    let p2p_id = String::from("test_p2p_id");

    // Make admin a reporter
    contract.add_reporter(accounts.alice).unwrap();

    // Should report only for listed DDC node
    assert_eq!(
        contract.report_ddn_status(p2p_id.clone(), true),
        Err(Error::DDNNotFound)
    );
}

#[ink::test]
fn report_ddn_status_works() {
    let mut contract = make_contract();
    let accounts = default_accounts::<DefaultEnvironment>().unwrap();
    let p2p_id = String::from("test_p2p_id");
    let url = String::from("test_url");

    // Make admin a reporter
    contract.add_reporter(accounts.alice).unwrap();

    // Add DDC node to the list
    contract.add_ddc_node(p2p_id.clone(), url).unwrap();

    // Should return Ok
    assert_eq!(contract.report_ddn_status(p2p_id.clone(), true), Ok(()));
}

#[ink::test]
fn default_ddn_status_works() {
    let mut contract = make_contract();
    let p2p_id = String::from("test_p2p_id");
    let url = String::from("test_url");
    let new_url = String::from("test_url_new");

    // Add DDC node to the list
    contract.add_ddc_node(p2p_id.clone(), url.clone()).unwrap();

    // Set new status
    contract.set_ddn_status(p2p_id.clone(), 2, false).unwrap();
    contract.set_ddn_status(p2p_id.clone(), 6, false).unwrap();

    // Repeat adding DDC node (update url)
    contract
        .add_ddc_node(p2p_id.clone(), new_url.clone())
        .unwrap();

    // Get updated status
    assert_eq!(
        contract.get_ddn_status(p2p_id.clone()),
        Ok(DDNStatus {
            is_online: false,
            total_downtime: 4,
            reference_timestamp: 0,
            last_timestamp: 6,
        })
    );
}

#[ink::test]
fn report_metrics_updates_ddn_status_works() {
    let mut contract = make_contract();
    let accounts = default_accounts::<DefaultEnvironment>().unwrap();

    let first_day = 1000;

    let today_ms = (first_day + 17) * MS_PER_DAY;
    let ddn_id = b"12D3KooWPfi9EtgoZHFnHh1at85mdZJtj7L8n94g6LFk6e8EEk2b".to_vec();
    let stored_bytes = 99;
    let wcu = 999;
    let rcu = 999;

    let p2p_id = String::from_utf8(ddn_id.clone()).unwrap();
    let url = String::from("test_url");

    // Add DDC node to the list
    contract.add_ddc_node(p2p_id.clone(), url).unwrap();

    // Set new DDC node status
    contract.set_ddn_status(p2p_id.clone(), 0, false).unwrap();

    // Make admin a reporter
    contract.add_reporter(accounts.alice).unwrap();

    // Report DDN metrics
    contract
        .report_metrics_ddn(ddn_id.clone(), today_ms, stored_bytes, wcu, rcu)
        .unwrap();

    // DDN status should be online
    assert_eq!(
        contract.get_ddn_status(p2p_id),
        Ok(DDNStatus {
            is_online: true,
            total_downtime: 0,
            reference_timestamp: 0,
            last_timestamp: 0,
        })
    );
}

// ---- Metrics Reporting ----
#[ink::test]
fn is_within_limit_works_outside_limit() {
    let mut contract = make_contract();
    let accounts = default_accounts::<DefaultEnvironment>().unwrap();
    let app_id = accounts.alice;
    let metrics = MetricValue {
        start_ms: 0,
        storage_bytes: 99999,
        wcu_used: 10,
        rcu_used: 10,
    };

    let some_day = 0;
    let ms_per_day = 24 * 3600 * 1000;

    let today_ms = some_day * ms_per_day;

    contract.subscribe(1).unwrap();

    assert_eq!(contract.is_within_limit(app_id), true);

    contract.add_reporter(accounts.alice).unwrap();
    contract
        .report_metrics(app_id, today_ms, metrics.storage_bytes, metrics.wcu_used, metrics.rcu_used)
        .unwrap();

    assert_eq!(contract.is_within_limit(app_id), false)
}

#[ink::test]
fn is_within_limit_works_within_limit() {
    let mut contract = make_contract();
    let accounts = default_accounts::<DefaultEnvironment>().unwrap();
    let app_id = accounts.alice;
    let metrics = MetricValue {
        start_ms: 0,
        storage_bytes: 5,
        wcu_used: 10,
        rcu_used: 10,
    };
    let some_day = 9999;
    let ms_per_day = 24 * 3600 * 1000;

    let today_ms = some_day * ms_per_day;

    contract.subscribe(1).unwrap();

    contract.add_reporter(accounts.alice).unwrap();
    contract
        .report_metrics(app_id, today_ms, metrics.storage_bytes, metrics.wcu_used, metrics.rcu_used)
        .unwrap();

    assert_eq!(contract.is_within_limit(app_id), true)
}

#[ink::test]
fn report_metrics_ddn_works() {
    let mut contract = make_contract();
    let accounts = default_accounts::<DefaultEnvironment>().unwrap();

    let first_day = 1000;

    let today_ms = (first_day + 17) * MS_PER_DAY;
    let ddn_id = b"12D3KooWPfi9EtgoZHFnHh1at85mdZJtj7L8n94g6LFk6e8EEk2b".to_vec();
    let storage_bytes = 99;
    let wcu_used = 999;
    let rcu_used = 999;

    let p2p_id = String::from_utf8(ddn_id.clone()).unwrap();
    let url = String::from("test_url");

    contract.add_ddc_node(p2p_id, url).unwrap();

    contract.add_reporter(accounts.alice).unwrap();
    contract
        .report_metrics_ddn(ddn_id.clone(), today_ms, storage_bytes, wcu_used, rcu_used)
        .unwrap();

    let last_day_inclusive = first_day + PERIOD_DAYS - 1;
    let now_ms = last_day_inclusive * MS_PER_DAY + 12345;
    let result = contract.metrics_for_ddn_at_time(ddn_id, now_ms);

    let mut expected = vec![
        MetricValue {
            start_ms: 0,
            storage_bytes: 0,
            wcu_used: 0,
            rcu_used: 0,
        };
        PERIOD_DAYS as usize
    ];

    for i in 0..PERIOD_DAYS as usize {
        expected[i].start_ms = (first_day + i as u64) * MS_PER_DAY;
    }

    expected[17].storage_bytes = storage_bytes;
    expected[17].wcu_used = wcu_used;
    expected[17].rcu_used = rcu_used;

    assert_eq!(result, expected);
}

#[ink::test]
fn get_app_limit_works() {
    let mut contract = make_contract();
    let accounts = default_accounts::<DefaultEnvironment>().unwrap();
    let app_id = accounts.alice;
    let now = 0;
    let later = now + 45 * MS_PER_DAY;

    assert_eq!(
        contract.get_app_limit_at_time(app_id, 0),
        Err(Error::NoSubscription)
    );

    contract.subscribe(2).unwrap();

    assert_eq!(
        contract.get_app_limit_at_time(app_id, 0),
        Ok(AppSubscriptionLimit::new(
            4000,
            4000,
            4000,
        ))
    );

    assert_eq!(
        contract.get_app_limit_at_time(app_id, later),
        Err(NoFreeTier)
    );


    contract.add_tier(0, 1000, 1000, 1000).unwrap();

    assert_eq!(
        contract.get_app_limit_at_time(app_id, later),
        Ok(AppSubscriptionLimit::new(
            1000,
            1000,
            1000,
        ))
    );
}