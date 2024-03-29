//import libs
use cosmwasm_std::{
    BankMsg, coin, Decimal, DepsMut, entry_point, Env, MessageInfo, Response, StdError, StdResult, Uint128
};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

// define init message struct
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {
    pub owner: String, // 合約擁有者地址
    pub base_interest_rate: Decimal, // 基礎年利率
}

// define contract supported operations
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ExecuteMsg {
    DepositCollateral { token_address: String, amount: Uint128 }, // 存入抵押品
    WithdrawCollateral { token_address: String, amount: Uint128 }, // 取出抵押品
    Borrow { amount: Uint128 }, // 借款
    RepayLoan { amount: Uint128 }, // 還款
}

// config and status
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    pub owner: String, //擁有者地址
    pub base_interest_rate: Decimal, //基礎年利率
}

// loan info
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LoanInfo {
    pub amount_borrowed: Uint128,//borrowed amount
    pub interest_rate: Decimal, //interest rate
    pub loan_start_time: u64, //loan start time
}

// Collateral info
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Collateral {
    pub token_address: String, //token address
    pub amount: Uint128, //amount
}

//storage config、loan info and collateral storage。
const CONFIG: Item<Config> = Item::new("config");
const LOANS: Map<String, LoanInfo> = Map::new("loans");
const COLLATERALS: Map<String, Collateral> = Map::new("collaterals");

// contract init
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        owner: msg.owner,
        base_interest_rate: msg.base_interest_rate,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

// execute contract operations
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::DepositCollateral { token_address, amount } => {
            deposit_collateral(deps, info, token_address, amount)
        },
        ExecuteMsg::WithdrawCollateral { token_address, amount } => {
            withdraw_collateral(deps, info, token_address, amount)
        },
        ExecuteMsg::Borrow { amount } => {
            borrow(deps, env, info, amount)
        },
        ExecuteMsg::RepayLoan { amount } => {
            repay_loan(deps, info, amount)
        },
    }
}

// deposit collateral logic
fn deposit_collateral(deps: DepsMut, info: MessageInfo, token_address: String, amount: Uint128) -> StdResult<Response> {
    if amount.is_zero() {
        return Err(StdError::generic_err("Amount cannot be zero"));
    }
    let collateral = Collateral { token_address, amount };
    COLLATERALS.save(deps.storage, info.sender.to_string(), &collateral)?;
    Ok(Response::new()
        .add_attribute("action", "deposit_collateral")
        .add_attribute("amount", amount.to_string()))
}

// withdraw collateral logic
fn withdraw_collateral(deps: DepsMut, info: MessageInfo, token_address: String, amount: Uint128) -> StdResult<Response> {
    // 首先檢查用戶是否有足夠的抵押品可供取出
    let collateral = COLLATERALS.load(deps.storage, info.sender.to_string())?;
    if collateral.token_address != token_address || collateral.amount < amount {
        return Err(StdError::generic_err("Insufficient collateral or mismatched token address"));
    }

    // 更新抵押品的狀態
    if collateral.amount == amount {
        // 如果取出的數量等於總抵押量，則從存儲中移除該抵押品記錄
        COLLATERALS.remove(deps.storage, info.sender.to_string());
    } else {
        // 否則更新存儲的抵押品數量
        let updated_collateral = Collateral {
            token_address: collateral.token_address,
            amount: collateral.amount - amount,
        };
        COLLATERALS.save(deps.storage, info.sender.to_string(), &updated_collateral)?;
    }

    // 模擬將抵押品返回給用戶的過程（在實際合約中，這可能涉及調用其他合約或處理特定的資產轉移邏輯）
    // 這裡僅示範將操作結果作為響應屬性返回
    Ok(Response::new()
        .add_attribute("action", "withdraw_collateral")
        .add_attribute("amount", amount.to_string())
        .add_attribute("token_address", token_address))
}

// borrow logic
fn borrow(deps: DepsMut, env: Env, info: MessageInfo, amount: Uint128) -> StdResult<Response> {
    let loan_info = LoanInfo {
        amount_borrowed: amount,
        interest_rate: Decimal::percent(5), // Assumes a fixed annual interest rate of 5%
        loan_start_time: env.block.time.seconds(),
    };
    LOANS.save(deps.storage, info.sender.to_string(), &loan_info)?;

    let payout = coin(amount.u128(), "usdc"); // Example assumes "usdc" as the currency
    let bank_msg = BankMsg::Send {
        to_address: info.sender.into(),
        amount: vec![payout],
    };

    Ok(Response::new()
        .add_message(bank_msg)
        .add_attribute("action", "borrow")
        .add_attribute("amount", amount.to_string()))
}

// repay logic
fn repay_loan(deps: DepsMut, info: MessageInfo, amount: Uint128) -> StdResult<Response> {
    let loan = LOANS.load(deps.storage, info.sender.to_string())?;
    let interest = loan.amount_borrowed * loan.interest_rate;
    let total_due = loan.amount_borrowed + interest;

    if amount < total_due {
        return Err(StdError::generic_err("Repayment amount is not enough to cover the loan and interest"));
    }
    LOANS.remove(deps.storage, info.sender.to_string());

    Ok(Response::new()
        .add_attribute("action", "repay_loan")
        .add_attribute("amount", amount.to_string())
        .add_attribute("interest_paid", interest.to_string()))
}

// Implements interest rate update logic (owner only)
fn update_interest_rate(deps: DepsMut, info: MessageInfo, new_rate: Decimal) -> StdResult<Response> {
    // Verify if the sender is the owner
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(StdError::generic_err("You have no permissions."));
    }

    // Update the interest rate
    CONFIG.update(deps.storage, |mut conf| -> StdResult<_> {
        conf.base_interest_rate = new_rate;
        Ok(conf)
    })?;

    Ok(Response::new()
        .add_attribute("action", "update_interest_rate")
        .add_attribute("new_rate", new_rate.to_string()))
}


//Possible Issues:
//
// Permission Control:
// The contract does not explicitly control permissions for certain operations
// (such as changing interest rates or withdrawing collateral), potentially allowing anyone to
// perform these actions. Typically, these operations should be restricted so that only
// the contract owner or users with specific permissions can execute them.
//
// Interest Rate Updates:
// The contract lacks functionality to update interest rates.
// In practical applications, it may be necessary to adjust the base interest rate based on
// market conditions.
//
// Collateral Handling:
// In the withdraw_collateral function, if a user attempts to
// withdraw an amount of collateral exceeding the stored amount, the contract will return an error.
// This behavior is expected, but the contract does not explicitly handle the situation where a
// borrower has an outstanding loan. In real-world applications, borrowers should be prevented from
// withdrawing collateral while having outstanding loans, or they should be required to repay first.
//
// Error Handling:
// Some functions may require more detailed error messages when handling errors,
// to aid in debugging and help users understand why an operation failed.
//
// Security:
// The contract does not address security considerations,
// such as integer overflow or re-entrancy attacks. Although CosmWasm has
// certain security mechanisms in place, it is best to explicitly handle potential
// security risks within the contract logic.
//
// Loan and Repayment Details:
// The contract simplifies the loan and repayment process and does not account
// for complex scenarios such as loan terms and overdue repayments.
