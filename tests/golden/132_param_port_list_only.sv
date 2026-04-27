module pkg_only #(parameter int X = 1,
                  parameter int Y = 2);
endmodule
// expected -----
module pkg_only #(
  parameter int X = 1,
  parameter int Y = 2
);
endmodule
