# BlueGrouse: A toolkit for Postgres migrations 

The only way to re-arrange column order on Postgres tables is by dropping a table, then recreating it. 
<br>This takes a TON of time if the table has many dependent objects (Views, FKs, etc).
<br>BlueGrouse automates this process by generating the scripts required to do these "intra-table migrations". It also provides a suite of exploratory tools which can be used to quickly understand how a table is used/referenced in a database.