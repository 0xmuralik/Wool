use std::collections::HashMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::{entry_point,to_binary,attr,Addr,CosmosMsg,BankMsg, Uint128,Binary,Coin, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::error::ContractError;
use crate::msg::{FundsResponse,PriceResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Pool, POOLS};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreatePool{name} => try_create(deps,info,name),
        ExecuteMsg::AddFunds{pool_id} => try_add_funds(deps,info,pool_id),
        ExecuteMsg::SwapTokens{pool_id,amount} => try_swap_tokens(deps,info,pool_id,amount),
    }
}

pub fn try_create(
    deps: DepsMut,
    info:MessageInfo,
    name:String
) -> Result<Response, ContractError> {
    if info.funds.len()!=2{
        return Err(ContractError::FundsMismatched{
            // expected:String::from("2 coins"),
            // found:info.funds.len().to_string(),
        });
    }

    // TODO: add automatic id creation (id=name#number) ex: akt/regen#23
    let new_pool= Pool{
        id: name.clone(),
        name:name.clone(),
        coins:info.funds
    };

    POOLS.update(deps.storage, name.as_str(), |existing| match existing {
        None => Ok(new_pool),
        Some(_) => Err(ContractError::AlreadyInUse {}),
    })?;
    Ok(Response::default())
}

pub fn try_add_funds(
    deps: DepsMut,
    info:MessageInfo,
    pool_id:String
) -> Result<Response, ContractError> {

    let mut pool=POOLS.load(deps.storage, pool_id.as_str())?;
    
    if info.funds.len()!=pool.coins.len(){
        return Err(ContractError::FundsMismatched{
            // expected: pool.coins.len().to_string(),
            // found: info.funds.len().to_string(),
        });
    }
    let mut token_ids= HashMap::new();
    for token in info.funds{
        let n=pool.clone().coins.iter().enumerate().find_map(|(i, exist)| {
            if exist.denom == token.denom {
                // pool.coins[i].amount += token.amount;
                // Some(pool.coins[i].amount)
                Some(i)
            } else {
                None
            }
        });

        match n{
            Some(id)=> {
                token_ids.insert(id, token);
            }
            None => return Err(ContractError::FundsMismatched{
                // expected: "coins in pool".to_string(),
                // found: token.denom
            })
        }

    }
    let mut res = Response::new();
    res.add_attribute("pool_id", pool_id.as_str());
    res.add_attribute("action", "add funds");
    for (id,token) in token_ids {
        pool.coins[id].amount += token.amount;
        res.add_attribute("amount", token.amount);
        res.add_attribute("denom", token.denom);
    }
    POOLS.save(deps.storage,pool_id.as_str(),&pool)?;
    
    Ok(res)
}

pub fn try_swap_tokens(
    deps: DepsMut,
    info:MessageInfo,
    pool_id:String,
    amount: Coin,
) -> Result<Response, ContractError> {
    let mut pool=POOLS.load(deps.storage, pool_id.as_str())?;
    if info.funds.len()!=1{
        return Err(ContractError::FundsMismatched{});
    }

    // TODO: add error handling for token not found
    let give_coin_index=pool.coins.iter().enumerate().find_map(|(i,token)|{
        if token.denom==amount.denom{
            Some(i)
        }
        else{
            None
        }
    }).unwrap();

    if pool.coins[give_coin_index].amount < amount.amount{
        return Err(ContractError::InsufficientFunds{});
    }

    // TODO: add error handling for token not found
    let take_coin_index=pool.coins.iter().enumerate().find_map(|(i,token)|{
        if token.denom==info.funds[0].denom{
            Some(i)
        }
        else{
            None
        }
    }).unwrap();

    let price = query_price(deps.as_ref(), pool_id.clone(), amount.denom.clone(), info.funds[0].denom.clone()).unwrap();

    if amount.amount.checked_mul(price.price).unwrap()>info.funds[0].amount{
        return Err(ContractError::InsufficientFunds{});
    }

    pool.coins[take_coin_index].amount+=info.funds[0].amount;
    pool.coins[give_coin_index].amount=pool.coins[give_coin_index].amount.checked_sub(amount.amount).unwrap();

    POOLS.save(deps.storage,pool_id.as_str(),&pool)?;

    Ok(send_tokens(info.sender, vec![amount], "Swap"))
}

fn send_tokens(to_address: Addr, amount: Vec<Coin>, action: &str) -> Response {
    let attributes = vec![attr("action", action), attr("to", to_address.clone())];

    Response {
        submessages: vec![],
        messages: vec![CosmosMsg::Bank(BankMsg::Send {
            to_address: to_address.into(),
            amount,
        })],
        data: None,
        attributes,
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetFunds {id} => to_binary(&query_funds(deps,id)?),
        QueryMsg::GetPrice{pool_id,denom1,denom2}=>to_binary(&query_price(deps,pool_id,denom1,denom2)?),
    }
}

fn query_funds(deps: Deps,id:String) -> StdResult<FundsResponse> {
    let pool = POOLS.load(deps.storage ,id.as_str())?;
    Ok(FundsResponse { funds: pool.coins })
}

fn query_price(
    deps:Deps,
    pool_id:String,
    denom1:String,
    denom2:String,
)->StdResult<PriceResponse>{
    let pool = POOLS.load(deps.storage ,pool_id.as_str())?;
    let mut t1=Uint128(0);
    let mut t2=Uint128(0);

    for coin in pool.coins{
        if coin.denom==denom1{
            t1=coin.amount;
        } 
        else if coin.denom==denom2{
            t2=coin.amount;
        }
    }
    let price = t2.checked_div(t1)?;
    Ok(PriceResponse{price:price})
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins,coin, from_binary};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(0, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn create() {
        let mut deps = mock_dependencies(&coins(0, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(0, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // creating new pool
        let amount = vec![coin(200, "earth"),coin(100, "mars")];
        let info = mock_info("elon", &amount);
        let msg = ExecuteMsg::CreatePool {name:"POOL1".to_string()};
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // should show pool balances
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetFunds {id:"POOL1".to_string()}).unwrap();
        let value: FundsResponse = from_binary(&res).unwrap();
        assert_eq!(vec![coin(200, "earth"),coin(100, "mars")], value.funds);
    }

    #[test]
    fn add_funds() {
        let mut deps = mock_dependencies(&coins(0, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(0, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // creating new pool
        let amount = vec![coin(200, "earth"),coin(100, "mars")];
        let info = mock_info("elon", &amount);
        let msg = ExecuteMsg::CreatePool {name:"POOL1".to_string()};
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // should show pool balances
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetFunds {id:"POOL1".to_string()}).unwrap();
        let value: FundsResponse = from_binary(&res).unwrap();
        assert_eq!(vec![coin(200, "earth"),coin(100, "mars")], value.funds);

        // mismatched number of tokens
        let one_token_info = mock_info("murali", &coins(10, "earth"));
        let msg = ExecuteMsg::AddFunds { pool_id:  "POOL1".to_string()};
        let res = execute(deps.as_mut(), mock_env(), one_token_info, msg);
        match res {
            Err(ContractError::FundsMismatched {}) => {}
            _ => panic!("Must return Mismatched error"),
        }

        // mismatched type of tokens
        let amount = vec![coin(20, "earth"),coin(10, "moon")];
        let diff_token_info = mock_info("murali", &amount);
        let msg = ExecuteMsg::AddFunds { pool_id:  "POOL1".to_string()};
        let res = execute(deps.as_mut(), mock_env(), diff_token_info, msg);
        match res {
            Err(ContractError::FundsMismatched {}) => {}
            _ => panic!("Must return Mismatched error"),
        }

        // correct type and number of tokens
        let amount = vec![coin(20, "earth"),coin(10, "mars")];
        let add_fund_info = mock_info("murali", &amount);
        let msg = ExecuteMsg::AddFunds { pool_id: "POOL1".to_string() };
        let _res = execute(deps.as_mut(), mock_env(), add_fund_info, msg).unwrap();

        //query funds
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetFunds {id:"POOL1".to_string()}).unwrap();
        let value: FundsResponse = from_binary(&res).unwrap();
        assert_eq!(vec![coin(220, "earth"),coin(110, "mars")], value.funds);
    }

    #[test]
    fn price() {
        let mut deps = mock_dependencies(&coins(0, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(0, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // creating new pool
        let amount = vec![coin(200, "earth"),coin(100, "mars")];
        let info = mock_info("elon", &amount);
        let msg = ExecuteMsg::CreatePool {name:"POOL1".to_string()};
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // should show pool balances
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetFunds {id:"POOL1".to_string()}).unwrap();
        let value: FundsResponse = from_binary(&res).unwrap();
        assert_eq!(vec![coin(200, "earth"),coin(100, "mars")], value.funds);

        //query price
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetPrice{pool_id:"POOL1".to_string(),denom1:"earth".to_string(),denom2:"mars".to_string()}).unwrap();
        let value: PriceResponse = from_binary(&res).unwrap();
        assert_eq!(Uint128(0), value.price);
    }

    #[test]
    fn swap() {
        let mut deps = mock_dependencies(&coins(0, "token"));

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(0, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // creating new pool
        let amount = vec![coin(200, "earth"),coin(100, "mars")];
        let info = mock_info("elon", &amount);
        let msg = ExecuteMsg::CreatePool {name:"POOL1".to_string()};
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // should show pool balances
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetFunds {id:"POOL1".to_string()}).unwrap();
        let value: FundsResponse = from_binary(&res).unwrap();
        assert_eq!(vec![coin(200, "earth"),coin(100, "mars")], value.funds);

        //query price
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetPrice{pool_id:"POOL1".to_string(),denom1:"mars".to_string(),denom2:"earth".to_string()}).unwrap();
        let value: PriceResponse = from_binary(&res).unwrap();
        assert_eq!(Uint128(2), value.price);


        //wrong number of funds
        let ask = coin(1,"mars");
        let amount = vec![coin(200, "earth"),coin(100, "token")];
        let info = mock_info("elon", &amount);
        let msg = ExecuteMsg::SwapTokens{pool_id:"POOL1".to_string(),amount:ask};
        let res= execute(deps.as_mut(), mock_env(), info, msg);
        match res{
            Err(ContractError::FundsMismatched {}) => {}
            _ => panic!("Must return Insufficient funds error"),
        }

        //insufficent tokens sent
        let ask = coin(1,"mars");
        let info = mock_info("elon", &coins(1,"earth"));
        let msg = ExecuteMsg::SwapTokens{pool_id:"POOL1".to_string(),amount:ask};
        let res= execute(deps.as_mut(), mock_env(), info, msg);
        match res{
            Err(ContractError::InsufficientFunds {}) => {}
            _ => panic!("Must return Insufficient funds error"),
        }

        //insufficient tokens asked
        let ask = coin(300,"mars");
        let info = mock_info("elon", &coins(2,"earth"));
        let msg = ExecuteMsg::SwapTokens{pool_id:"POOL1".to_string(),amount:ask};
        let res= execute(deps.as_mut(), mock_env(), info, msg);
        match res{
            Err(ContractError::InsufficientFunds {}) => {}
            _ => panic!("Must return Insufficient funds error"),
        }
        
        let ask = coin(1,"mars");
        let info = mock_info("elon", &coins(2,"earth"));
        let msg = ExecuteMsg::SwapTokens{pool_id:"POOL1".to_string(),amount:ask};
        let res= execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(1, res.messages.len());
        let msg = res.messages.get(0).expect("no message");
        assert_eq!(
            msg,
            &CosmosMsg::Bank(BankMsg::Send {
                to_address: "elon".into(),
                amount: coins(1, "mars"),
            })
        );

        //qurey funds
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetFunds {id:"POOL1".to_string()}).unwrap();
        let value: FundsResponse = from_binary(&res).unwrap();
        assert_eq!(vec![coin(202, "earth"),coin(99, "mars")], value.funds);

        //qurey price
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetPrice{pool_id:"POOL1".to_string(),denom1:"mars".to_string(),denom2:"earth".to_string()}).unwrap();
        let value: PriceResponse = from_binary(&res).unwrap();
        assert_eq!(Uint128(2), value.price);
        
    }
}
