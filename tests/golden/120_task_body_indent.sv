module m;
  task   say(input string msg);
    $display("%s",msg);
  endtask
  task   step(input int n);
    int   k;
    k = n * 2;
    $display("k=%0d", k);
  endtask
endmodule
// expected -----
module m;
  task say(input string msg);
    $display("%s", msg);
  endtask
  task step(input int n);
    int k;
    k = n * 2;
    $display("k=%0d", k);
  endtask
endmodule
