local now = tonumber(redis.call('TIME')[1])
local day = math.floor(now / 86400)
local previous = tonumber(redis.call('HGET', KEYS[1], 'day') or '-1')
local count = tonumber(redis.call('HGET', KEYS[1], 'count') or '0')
if previous ~= day then count = 0 end
redis.call('ZREMRANGEBYSCORE', KEYS[2], '-inf', now - 900)
if count >= 180 or redis.call('ZCARD', KEYS[2]) >= 30 or redis.call('EXISTS', KEYS[3]) == 1 then return 0 end
count = count + 1
redis.call('HSET', KEYS[1], 'day', day, 'count', count)
redis.call('EXPIRE', KEYS[1], 172800)
redis.call('ZADD', KEYS[2], now, tostring(day) .. ':' .. tostring(count))
redis.call('EXPIRE', KEYS[2], 901)
redis.call('SET', KEYS[3], '1', 'EX', 1200)
return 1
