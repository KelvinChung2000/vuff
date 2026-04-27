module m;
  import pkg::*;
  import pkg::byte_t;
  import other_pkg::name1, other_pkg::name2;
endmodule
// expected -----
module m;
  import pkg::*;
  import pkg::byte_t;
  import other_pkg::name1, other_pkg::name2;
endmodule
