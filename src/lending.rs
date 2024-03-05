use cosmwasm_std::{
    attr, BankMsg, coin, CosmosMsg, Decimal, DepsMut, entry_point, Env, MessageInfo, Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

// 定義初始化合約的訊息結構
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {
    pub owner: String, // 合約擁有者地址
    pub base_interest_rate: Decimal, // 基礎年利率
}

// 定義合約操作的訊息種類
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ExecuteMsg {
    DepositCollateral { token_address: String, amount: Uint128 }, // 存入抵押品
    WithdrawCollateral { token_address: String, amount: Uint128 }, // 取出抵押品
    Borrow { amount: Uint128 }, // 借款
    RepayLoan { amount: Uint128 }, // 還款
}

// 合約配置和狀態
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    pub owner: String,
    pub base_interest_rate: Decimal,
}

// 借款資訊
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LoanInfo {
    pub amount_borrowed: Uint128,
    pub interest_rate: Decimal,
    pub loan_start_time: u64,
}

// 抵押品資訊
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Collateral {
    pub token_address: String,
    pub amount: Uint128,
}

const CONFIG: Item<Config> = Item::new("config");
const LOANS: Map<String, LoanInfo> = Map::new("loans");
const COLLATERALS: Map<String, Collateral> = Map::new("collaterals");

// 初始化合約
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

// 執行合約操作
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

// 實作存入抵押品的邏輯
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

// 實作取出抵押品的邏輯
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

// 實作借款的邏輯
fn borrow(deps: DepsMut, env: Env, info: MessageInfo, amount: Uint128) -> StdResult<Response> {
    let loan_info = LoanInfo {
        amount_borrowed: amount,
        interest_rate: Decimal::percent(5), // Assumes a fixed annual interest rate of 5%
        loan_start_time: env.block.time.seconds(),
    };
    LOANS.save(deps.storage, info.sender.to_string(), &loan_info)?;

    let payout = coin(amount.u128(), "uscrt"); // Example assumes "uscrt" as the currency
    let bank_msg = BankMsg::Send {
        to_address: info.sender.into(),
        amount: vec![payout],
    };

    Ok(Response::new()
        .add_message(bank_msg)
        .add_attribute("action", "borrow")
        .add_attribute("amount", amount.to_string()))
}

// 實作還款的邏輯
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
