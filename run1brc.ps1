# Define the path to the program/executable

# Record the start time

	$startTime = Get-Date

	$Command = ".\target\release\one-billion-row-challenge.exe "

	$param1=$args[0]

# Execute the program
	& .\target\release\one-billion-row-challenge.exe

# Record the end time
	$endTime = Get-Date

# Calculate the time difference in milliseconds
	$timeTaken = $timeTaken + ($endTime - $startTime).TotalMilliseconds

# Print the result
Write-Host "Time taken: $($timeTaken) milliseconds"

