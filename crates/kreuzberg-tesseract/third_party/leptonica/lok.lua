--[[
	Modify Leptonica's *.c file functions returning
	"0 if OK, 1 on error" to use the l_ok typedef.
--]]

debug = false

---
-- copy a file src to dst
-- \param src source filename
-- \param dst destination filename
-- \param blocksize block size for copying (default 1M)
-- \return true on success; nil,error on error
function copyfile(src, dst, blocksize)
	blocksize = blocksize or 1024*1024
	local fs, fd, err
	-- return after closing the file descriptors
	local function ret(...)
		if fs then fs:close() end
		if fd then fd:close() end
		return ...
	end
	fs, err = io.open(src)
	if not fs then return ret(nil, err) end
	fd, err = io.open(dst, "wb")
	if not fd then return ret(nil, err) end
	while true do
		local ok, data
		data = fs:read(blocksize)
		if not data then break end
		ok, err = fd:write(data)
		if not ok then return ret(nil, err) end
	end
	return ret(true)
end

---
-- List the array table (t) contents to fd.
-- \param fd io file descriptor
-- \param t array table to list
function list(fd, t)
	if t ~= nil then
		for _,l in ipairs(t) do
			fd:write(l .. '\n')
		end
	end
end

---
--- Modify the file fs and write the result to fd.
-- \param fs source file stream
-- \param fd destination file stream
-- \return true on success
function modify(fs, fd)
	local state = 0		-- parsing state
	local to_l_ok = false	-- true, if the l_int32 return type should be changed
	local b_file = false	-- true, have seen a \file comment

	while true do
		line = fs:read()
		if line == nil then
			-- end of file
			break
		end

		if line:match('^/%*[!*]$') then
			-- start of Doxygen comment
			-- introduces a new function
			-- go to state 1 (inside doxygen comment)
			state = 1
		end

		if state == 3 then
			-- 2nd line after a comment
			-- contains the name of the function
			-- go to state 4 (skip until end of function)
			state = 4
		end

		if state == 2 then
			-- 1st line after a comment
			-- contains the return type
			if to_l_ok and line == 'l_int32' then
				line = 'l_ok'
			end
			if b_file then
				-- back to state 0 (look for doxygen comment)
				state = 0
				to_l_ok = false
				b_file = false
			else
				-- go to state 3 (2nd line after doxygen comment)
				state = 3
			end
		end

		if line == ' */' then
			-- end of Doxygen comment
			-- go to state 2 (1st line after doxygen comment)
			state = 2
		end

		if state == 1 then
			-- inside doxygen comment
			if line:match("%\\return%s+0 if OK") then
				-- magic words that indicate l_ok return type
				to_l_ok = true
			end
			if line:match("%\\file%s+") then
				-- this is a file comment
				b_file = true
			end
		end

		if debug then
			if to_l_ok then
				print("[" .. state .. ";1] " .. line)
			else
				print("[" .. state .. ";0] " .. line)
			end
		end

		if state == 4 and line == '}' then
			-- end of a function
			-- print("name='" .. name .. "'")
			-- back to state 0 (look for doxygen comment)
			state = 0
			to_l_ok = false
			b_file = false
		end
		fd:write(line .. '\n')
	end
	return true
end

---
-- Main function.
-- \param arg table array of command line parameters

script = arg[0] or ""
if #arg < 1 then
	print("Usage: lua " .. script .." src/*.c")
end

for i = 1, #arg do
	local input = arg[i]
	local backup = input .. "~"
	local ok, err = copyfile(input, backup)
	if not ok then
		print("Error copying " .. input .." to " .. backup .. ": " ..err)
	end
	local fs = io.open(backup)
	local fd = io.open(input, "wb")
	modify(fs, fd)
	fd:close()
	fs:close()
end
