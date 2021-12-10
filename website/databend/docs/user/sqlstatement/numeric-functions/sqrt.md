---
title: SQRT
---

Returns the square root of a nonnegative number x.

## Syntax

```sql
SQRT(x)
```

## Arguments

| Arguments   | Description |
| ----------- | ----------- |
| x | The nonnegative numerical value. |

## Return Type

A Float64 data type value.


## Examples

```
mysql> SELECT SQRT(4);
+---------+
| SQRT(4) |
+---------+
|       2 |
+---------+
1 row in set (0.00 sec)

mysql> SELECT SQRT(-16);
+--------------+
| SQRT((- 16)) |
+--------------+
|         NULL |
+--------------+
1 row in set (0.00 sec)
```
