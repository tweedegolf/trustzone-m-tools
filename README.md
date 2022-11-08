# Trustzone-M tools

A set of crates that lets you easily use Trustzone and jump back and forth between a secure app and a nonsecure app.

Check out the example project to see a working setup. It's made to be run on an nRF9160-DK.

## Project layout

- `macros`: This is where the proc macros live. The implementation of them is done in the `tools` crate.
- `nonsecure-rt`: The runtime that the nonsecure app has to use. This replaces the `cortex-m-rt` crate. It has no main and no interrupt support (yet).
- `secure-rt`: The runtime for the secure app. This contains the code to do the trustzone setup and makes sure that the nonsecure app gets initialized.
- `tools`: The implementation of the macros live here as well as the bindings generator.

## TODO's: (help wanted 🙂)

- Interrupt support. All interrupts are on the secure side and it does not know about nonsecure interrupts.
  Right now you'll just have to make the interrupt on the secure side and then manually call the processing function on the nonsecure side.
  Any code that uses the `cortex-m-rt` interrupt macro won't work on the nonsecure side right now.
- nRF5340-app support. Should be relatively easy because it's almost the same as the already implemented nRF91.
- Other chips support.
- Chache veneer pointers. Currently they are always searched for, but this only has to happen the first time.
