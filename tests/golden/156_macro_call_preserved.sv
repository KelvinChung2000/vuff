`define assert(expr) empty_statement
module m;
  task empty_statement;
  endtask
  initial begin
    if (cond)
      `assert(p == q);
    if (other)
      `assert(reset_n);
  end
endmodule
// expected -----
`define assert(expr) empty_statement
module m;
  task empty_statement;
  endtask
  initial begin
    if (cond)
      `assert(p == q);
    if (other)
      `assert(reset_n);
  end
endmodule
