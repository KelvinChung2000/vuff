module m;
  initial begin
    obj . method ( arg );
    queue . push_back ( item );
  end
endmodule
// expected -----
module m;
  initial begin
    obj.method(arg);
    queue.push_back(item);
  end
endmodule
