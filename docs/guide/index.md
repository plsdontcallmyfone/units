# units

units is a token platform on Solana where a token's rules are **hooks you can own, trade and swap**, and where tokens can **act on each other**.

Every trade, transfer and swap of a units token runs through the hooks equipped in its slots. Each hook is an item with an owner. When it runs, the owner earns.

> Status: built and tested in the program test suite. Not deployed to any cluster yet. See [Security](security.md).

## Three ideas

### 1. Tokens with slots

A units token has up to four **slots**. Each slot holds one hook. The slot's limits (how much it may take, whether it may refuse, who may change it) are fixed at launch and never change. What sits inside the slot can change, by holder vote or by an on-chain performance rule.

[Tokens and slots](concepts/tokens-and-slots.md)

### 2. Hooks are assets

A hook is an **item**: a supply-1 token that wraps one audited template with its own parameters. Items have an owner, a level and a royalty. Every time an item runs on any token, its owner earns a share of what it collects. Items can be listed, sold, rented, forged and combined.

[Hooks and items](concepts/hooks-and-items.md) and [The hook economy](concepts/hook-economy.md)

### 3. Tokens that interact

Hooks can read other tokens' markets, see where a buyer came from, and pay into treaties both sides agreed to. Communities raid each other, besiege rivals with their war chests, sign treaties and compete in seasons.

[Relations](concepts/relations.md) and [War](concepts/war.md)

## Who it is for

| You are | You use units to |
| --- | --- |
| A launcher | Launch a token with a slot table, starting items and a war chest |
| A holder | Vote on which items your token runs, earn rewards, loot and rank |
| A trader | Trade, raid rival tokens for fee discounts, join raids in one transaction |
| A builder | Create items and templates, earn royalties every time they run |
| An agent operator | Run an AI agent with an on-chain passport that builds, brokers treaties and earns |

## Where to start

- New to units: [How a trade runs](concepts/how-a-trade-runs.md)
- Launching: [Launch a token](guides/launch-a-token.md)
- Building: [Build a hook template](guides/build-a-hook-template.md)
- Integrating: [Programs](reference/programs.md) and [Limits](reference/limits.md)
