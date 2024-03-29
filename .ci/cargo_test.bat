@echo OFF
git show --summary | findstr "Author" | findstr "Pipelines-Bot" > tmp.txt
set /p VAR11=<tmp.txt
IF "%VAR11%" equ "" (
  rustup update
  cargo test --all --jobs 1
  IF errorlevel 1 (
    EXIT /B 2
  )
) ELSE (
  IF "%1" equ "Schedule" (
    rustup update
    cargo test --all --jobs 1
    IF errorlevel 1 (
      EXIT /B 2
    )
  ) ELSE (
    IF "%1" equ "Manual" (
      rustup update
      cargo test --all --jobs 1
      IF errorlevel 1 (
        EXIT /B 2
      )
    )
  )
)
set "VAR11="
del tmp.txt
@echo ON
