# BlueGrouse: A tool for Postgres table re-arranging 

### The only way to re-arrange column order on Postgres tables is by dropping the table, then recreating it. This takes a TON of time if the table has many dependent objects (Views, FKs, etc). BlueGrouse automates this process.